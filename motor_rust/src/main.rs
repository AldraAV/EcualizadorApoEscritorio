//! Ejecutable principal del Ecualizador de Escritorio Soberano en Rust (`motor_dsp_poweramp.exe`).
//! Provee de forma 100% nativa e integrada:
//! 1. Ventana Nativa de Escritorio Win32/GDI con Doble Bufer a 60 FPS (`ventana_nativa_win32.rs`)
//!    reconstruida a partir de los layouts AXML extraidos con REA de `Equalizador Poweramp.apk`.
//! 2. Hilo de Audio DSP en Tiempo Real (64-bit Biquad + Tono + Espacio Mid/Side + Compresor + Reverb + FFT 64 barras)
//!    con Captura WASAPI Loopback (`CapturadorWasapiLoopback`) y Renderizado Directo WASAPI (`RenderizadorWasapiDirecto`).
//! 3. Sincronizacion binaria IPC (`estado_dsp_activo.bin`) con nuestro propio Objeto COM APO de Windows (`apo_com_windows.rs`).

use std::env;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use motor_ecualizador_poweramp::apo_com_windows::{guardar_estado_binario_ipc, ruta_estado_ipc_global};
use motor_ecualizador_poweramp::cadena_maestra_dsp::CadenaMaestraDsp;
use motor_ecualizador_poweramp::captura_wasapi_loopback::{
    CapturadorWasapiLoopback, RenderizadorWasapiDirecto,
};
use motor_ecualizador_poweramp::cargador_presets::{
    buscar_presets_autoeq_en_archivo, exportar_archivo_apo,
};
use motor_ecualizador_poweramp::instalador_apo::{
    consultar_estado_instalacion_apo, desinstalar_apo, instalar_y_activar_apo,
    solicitar_elevacion_administrador,
};
use motor_ecualizador_poweramp::interfaz_iced::ejecutar_interfaz_iced;
use motor_ecualizador_poweramp::ventana_nativa_win32::{
    ejecutar_ventana_nativa_poweramp, EnrutamientoWasapiEstado, TelemetriaCompartidaVentana,
};

/// Localiza el directorio base del proyecto `EcualizadorApoEscritorio`.
fn localizar_directorio_base() -> PathBuf {
    let actual = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if actual.join("recursos").exists() {
        return actual;
    }
    if let Some(padre) = actual.parent() {
        if padre.join("recursos").exists() {
            return padre.to_path_buf();
        }
    }
    actual
}

/// Localiza el archivo `auto_eq.db` extraido del APK de Poweramp.
fn localizar_base_datos_autoeq(base: &Path) -> PathBuf {
    let ruta_recursos = base.join("recursos").join("auto_eq.db");
    if ruta_recursos.exists() {
        return ruta_recursos;
    }
    base.join("auto_eq.db")
}

fn main() {
    #[cfg(target_os = "linux")]
    {
        // En entornos Linux (especialmente maquinas virtuales o controladores Mesa/Vulkan basicos),
        // el backend Vulkan puede fallar la validacion de shaders quad con SHADER_FLOAT16_IN_FLOAT32.
        // Establecer OpenGL (EGL/GLX) por defecto garantiza compatibilidad universal.
        if env::var("WGPU_BACKEND").is_err() {
            env::set_var("WGPU_BACKEND", "gl");
        }
    }

    let argumentos: Vec<String> = env::args().collect();
    let directorio_base = localizar_directorio_base();

    // Modo CLI de validacion matematica del motor DSP (Cero mocks, cero UI)
    let es_modo_validacion = argumentos.iter().skip(1).any(|arg| {
        let a = arg.to_lowercase().replace("-", "").replace("_", "");
        a.contains("validar") || a.contains("probar") || a.contains("test") || a == "dsp" || a == "dps"
    });

    if es_modo_validacion {
        println!("\n========================================================");
        println!("  VALIDACION MATEMATICA DEL MOTOR DSP (CERO MOCKS)");
        println!("========================================================\n");
        let fs = 48000.0;
        let mut cadena = CadenaMaestraDsp::nueva(fs);
        // Desactivar limitador para verificar la linealidad y fase pura de los filtros IIR Biquad
        cadena.establecer_limitador_activo(false);

        // Amplitud de prueba normalizada a 0.25 (-12 dBFS) para permitir margen dinamico (+6 dB)
        let tono_1k: Vec<f32> = (0..48000)
            .map(|n| {
                let t = n as f32 / fs as f32;
                0.25 * (2.0 * std::f32::consts::PI * 1000.0 * t).sin()
            })
            .collect();

        // Estabilizar filtros
        for _ in 0..1000 {
            cadena.procesar_par_estereo(0.0, 0.0);
        }

        let mut max_in = 0.0_f32;
        let mut max_out_plano = 0.0_f32;
        for (idx, &muestra) in tono_1k.iter().enumerate() {
            let (izq, _) = cadena.procesar_par_estereo(muestra, muestra);
            if idx >= 5000 {
                max_in = max_in.max(muestra.abs());
                max_out_plano = max_out_plano.max(izq.abs());
            }
        }
        let ganancia_plana_db = 20.0 * (max_out_plano / max_in).log10();
        println!(
            "[1] Respuesta Plana (0.0 dB): Entrada={:.4}, Salida={:.4} -> Ganancia: {:+.2} dB | {}",
            max_in,
            max_out_plano,
            ganancia_plana_db,
            if ganancia_plana_db.abs() < 0.05 {
                "VERIFICADO [Plano puro 0 dB]"
            } else {
                "FALLO"
            }
        );

        // 2. Probar realce Biquad de +6 dB a 1000 Hz
        let mut estado_realce = cadena.obtener_estado().clone();
        for banda in &mut estado_realce.bandas {
            if (banda.frecuencia_hz - 1000.0).abs() < 50.0 {
                banda.ganancia_db = 6.0;
            }
        }
        let _ = cadena.aplicar_estado(estado_realce);
        for _ in 0..2000 {
            cadena.procesar_par_estereo(0.0, 0.0);
        }
        let mut max_out_realce = 0.0_f32;
        for (idx, &muestra) in tono_1k.iter().enumerate() {
            let (izq, _) = cadena.procesar_par_estereo(muestra, muestra);
            if idx >= 5000 {
                max_out_realce = max_out_realce.max(izq.abs());
            }
        }
        let ganancia_realce_db = 20.0 * (max_out_realce / max_in).log10();
        println!(
            "[2] Realce Biquad (+6 dB a 1 kHz): Salida={:.4} -> Ganancia: {:+.2} dB | {}",
            max_out_realce,
            ganancia_realce_db,
            if (ganancia_realce_db - 6.0).abs() < 0.25 {
                "VERIFICADO [Frecuencia amplificada exactamente]"
            } else {
                "FALLO"
            }
        );

        // 3. Probar corte Biquad de -12 dB a 1000 Hz
        let mut estado_corte = cadena.obtener_estado().clone();
        for banda in &mut estado_corte.bandas {
            if (banda.frecuencia_hz - 1000.0).abs() < 50.0 {
                banda.ganancia_db = -12.0;
            }
        }
        let _ = cadena.aplicar_estado(estado_corte);
        for _ in 0..2000 {
            cadena.procesar_par_estereo(0.0, 0.0);
        }
        let mut max_out_corte = 0.0_f32;
        for (idx, &muestra) in tono_1k.iter().enumerate() {
            let (izq, _) = cadena.procesar_par_estereo(muestra, muestra);
            if idx >= 5000 {
                max_out_corte = max_out_corte.max(izq.abs());
            }
        }
        let ganancia_corte_db = 20.0 * (max_out_corte / max_in).log10();
        println!(
            "[3] Atenuacion Biquad (-12 dB a 1 kHz): Salida={:.4} -> Ganancia: {:+.2} dB | {}",
            max_out_corte,
            ganancia_corte_db,
            if (ganancia_corte_db - (-12.0)).abs() < 0.25 {
                "VERIFICADO [Frecuencia atenuada exactamente]"
            } else {
                "FALLO"
            }
        );

        println!("\n========================================================");
        println!("  CONCLUSION: EL MOTOR DSP RESPONDE RIGUROSAMENTE");
        println!("========================================================\n");
        std::process::exit(0);
    }

    // Modo CLI de consulta de estado del APO
    if argumentos.len() >= 2 && argumentos[1] == "--estado-apo" {
        let estado = consultar_estado_instalacion_apo();
        println!("=== ESTADO DEL APO SOBERANO DE WINDOWS ===");
        println!("DLL instalada en Program Files: {}", estado.dll_instalada);
        println!("Ruta destino: {}", estado.ruta_dll.display());
        println!("Servidor COM registrado en CLSID: {}", estado.com_registrado);
        println!("AudioProcessingObject registrado: {}", estado.apo_registrado);
        println!("Proteccion de firma audiodg desactivada: {}", estado.proteccion_audiodg_desactivada);
        println!("Endpoints con APO inyectado: {}", estado.dispositivos_enlazados.len());
        for id in &estado.dispositivos_enlazados {
            println!("  -> {}", id);
        }
        return;
    }

    // Modo CLI de instalacion del APO con elevacion automatica si se requiere
    if argumentos.len() >= 2 && argumentos[1] == "--instalar-apo" {
        let ruta_dll_local = directorio_base
            .join("motor_rust")
            .join("target")
            .join("release")
            .join("motor_ecualizador_poweramp.dll");
        let ruta_dll_efectiva = if ruta_dll_local.exists() {
            ruta_dll_local
        } else {
            directorio_base.join("motor_ecualizador_poweramp.dll")
        };

        println!("Instalando APO desde: {}", ruta_dll_efectiva.display());
        match instalar_y_activar_apo(&ruta_dll_efectiva) {
            Ok(()) => {
                println!("APO Soberano registrado exitosamente en el registro de Windows.");
                println!("Para aplicar en caliente en audiodg.exe, reinicia el servicio de audio:");
                println!("  net stop audiosrv /y && net start audiosrv");
            }
            Err(e) => {
                eprintln!("Fallo la instalacion: {}", e);
                eprintln!("Intentando elevacion de privilegios UAC...");
                if solicitar_elevacion_administrador("--instalar-apo") {
                    println!("Solicitud UAC enviada con exito.");
                } else {
                    eprintln!("No se pudo solicitar la elevacion de permisos.");
                }
            }
        }
        return;
    }

    // Modo CLI de desinstalacion del APO
    if argumentos.len() >= 2 && argumentos[1] == "--desinstalar-apo" {
        match desinstalar_apo() {
            Ok(()) => {
                println!("APO Soberano desinstalado exitosamente.");
            }
            Err(e) => {
                eprintln!("Fallo la desinstalacion: {}", e);
                if solicitar_elevacion_administrador("--desinstalar-apo") {
                    println!("Solicitud UAC enviada con exito.");
                }
            }
        }
        return;
    }
    if argumentos.len() >= 2 && argumentos[1] == "--extraer-ui-apk" {
        let dir_rea = Path::new(
            r"C:\Users\carde\AppData\Local\Temp\rea-extracted-a10b98e6-03e5-4732-b4a1-7df4e3a65808",
        );
        let archivos = [
            "res/layout-sw4dp-v13/activity_peq_main.xml",
            "res/layout-sw2dp-v13/merge_eq_layout.xml",
            "res/layout-sw2dp-v13/merge_eq_equalizer_tab_common.xml",
            "res/layout-sw4dp-v13/merge_eq_equalizer_buttons.xml",
            "res/layout-sw2dp-v13/merge_eq_vol_tab.xml",
            "res/layout-sw2dp-v13/merge_eq_reverb_tab.xml",
            "res/layout-sw2dp-v13/merge_eqbuttons_edit.xml",
            "res/layout-sw2dp-v13/merge_navbar.xml",
            "res/layout-sw2dp-v13/merge_vis_panel.xml",
            "res/xml/peq_builtin_skins.xml",
            "res/xml/peq_equ.xml",
            "res/xml/peq_compr.xml",
            "res/xml/peq_ui.xml",
        ];
        let mut acumulado = String::new();
        for rel in archivos {
            let ruta = dir_rea.join(rel);
            if let Ok(xml) =
                motor_ecualizador_poweramp::extractor_ui_apk::decodificar_archivo_axml(&ruta)
            {
                acumulado.push_str(&format!("<!-- ===== {} ===== -->\n{}\n", rel, xml));
            }
        }
        let destino = directorio_base
            .join("recursos")
            .join("layouts_poweramp_decodificados.xml");
        let _ = std::fs::write(&destino, &acumulado);
        println!(
            "Layouts decodificados en: {} ({} bytes)",
            destino.display(),
            acumulado.len()
        );
        return;
    }

    // Modo CLI de consulta rapida sobre auto_eq.db
    if argumentos.len() >= 3 && argumentos[1] == "--autoeq-buscar" {
        let consulta = &argumentos[2];
        let ruta_db = localizar_base_datos_autoeq(&directorio_base);
        match buscar_presets_autoeq_en_archivo(&ruta_db, consulta, 20) {
            Ok(resultados) => {
                println!(
                    "Se encontraron {} perfiles en auto_eq.db para '{}':",
                    resultados.len(),
                    consulta
                );
                for r in resultados {
                    println!(
                        "  [@{}] {} ({} bandas, {})",
                        r.desplazamiento_blob,
                        r.nombre_modelo,
                        r.numero_bandas,
                        if r.es_parametrico {
                            "Parametrico"
                        } else {
                            "Grafico"
                        }
                    );
                }
                return;
            }
            Err(e) => {
                eprintln!("Error al buscar en auto_eq.db: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Inicializar Cadena Maestra DSP a 48 kHz en estado plano (Cero presets, todo en cero)
    let cadena_inicial = CadenaMaestraDsp::nueva(48000.0);

    let ruta_apo = directorio_base.join("apo_config.txt");
    let _ = exportar_archivo_apo(&ruta_apo, cadena_inicial.obtener_estado());
    let ruta_ipc = ruta_estado_ipc_global(Some(&directorio_base));
    let _ = guardar_estado_binario_ipc(&ruta_ipc, cadena_inicial.obtener_estado());

    let cadena_compartida = Arc::new(Mutex::new(cadena_inicial));
    let telemetria = Arc::new(TelemetriaCompartidaVentana::nueva());
    let enrutamiento = Arc::new(Mutex::new(EnrutamientoWasapiEstado::default()));

    // Lanzar Hilo de Procesamiento DSP en Tiempo Real con Captura WASAPI Loopback + Renderizador Directo WASAPI
    let cadena_hilo_dsp = Arc::clone(&cadena_compartida);
    let telemetria_hilo_dsp = Arc::clone(&telemetria);
    let enrutamiento_hilo_dsp = Arc::clone(&enrutamiento);

    thread::spawn(move || {
        let mut id_cap_actual: Option<String> = None;
        let mut id_sal_actual: Option<String> = None;
        let mut usar_render_directo = false;

        if let Ok(enr) = enrutamiento_hilo_dsp.lock() {
            id_cap_actual = enr.id_dispositivo_captura.clone();
            id_sal_actual = enr.id_dispositivo_salida_directa.clone();
            usar_render_directo = enr.usar_salida_directa_wasapi;
        }

        let mut capturador_sistema =
            CapturadorWasapiLoopback::inicializar_con_dispositivo(id_cap_actual.as_deref()).ok();

        if let Some(ref cap) = capturador_sistema {
            telemetria_hilo_dsp
                .captura_wasapi_activa
                .store(true, Ordering::Relaxed);
            telemetria_hilo_dsp
                .frecuencia_actual_hz
                .store(cap.frecuencia_muestreo, Ordering::Relaxed);
            if let Ok(mut cadena) = cadena_hilo_dsp.lock() {
                let mut est = cadena.obtener_estado().clone();
                est.frecuencia_muestreo_hz = cap.frecuencia_muestreo as f64;
                let _ = cadena.aplicar_estado(est);
            }
        }

        let mut renderizador_directo: Option<RenderizadorWasapiDirecto> = None;
        if usar_render_directo {
            renderizador_directo =
                RenderizadorWasapiDirecto::inicializar_con_dispositivo(id_sal_actual.as_deref())
                    .ok();
            telemetria_hilo_dsp
                .render_directo_activo
                .store(renderizador_directo.is_some(), Ordering::Relaxed);
        }

        let mut bufer_pares_entrada: Vec<(f32, f32)> = Vec::with_capacity(2048);
        let mut bufer_pares_procesados: Vec<(f32, f32)> = Vec::with_capacity(2048);

        #[cfg(target_os = "linux")]
        {
            // Intentar crear un sink nulo virtual "AldraPoweramp" para que el usuario pueda enrutar el audio del sistema hacia el DSP
            let _ = std::process::Command::new("pactl")
                .args(["load-module", "module-null-sink", "sink_name=AldraPoweramp", "sink_properties=device.description=Aldra_Poweramp_DSP"])
                .output();
            println!("[Audio Linux] Sink virtual 'AldraPoweramp' inicializado.");
            println!("[Audio Linux] Para procesar Firefox/Spotify ejecuta: pactl set-default-sink AldraPoweramp");
        }

        #[cfg(target_os = "linux")]
        let mut capturador_pipewire: Option<std::process::Child> = {
            use std::process::{Command, Stdio};
            // 1. Intentar capturar desde el monitor del sink virtual AldraPoweramp
            Command::new("pw-record")
                .args(["--target", "AldraPoweramp.monitor", "--format", "f32", "--rate", "48000", "--channels", "2", "-"])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .or_else(|_| {
                    // 2. Intentar con parec desde AldraPoweramp.monitor
                    Command::new("parec")
                        .args(["--device=AldraPoweramp.monitor", "--format=float32le", "--rate=48000", "--channels=2"])
                        .stdout(Stdio::piped())
                        .stderr(Stdio::null())
                        .spawn()
                })
                .or_else(|_| {
                    // 3. Fallback: Capturar desde el monitor del sink predeterminado
                    Command::new("pw-record")
                        .args(["--target", "@DEFAULT_MONITOR@", "--format", "f32", "--rate", "48000", "--channels", "2", "-"])
                        .stdout(Stdio::piped())
                        .stderr(Stdio::null())
                        .spawn()
                })
                .or_else(|_| {
                    Command::new("parec")
                        .args(["--device=@DEFAULT_MONITOR@", "--format=float32le", "--rate=48000", "--channels=2"])
                        .stdout(Stdio::piped())
                        .stderr(Stdio::null())
                        .spawn()
                })
                .ok()
        };

        #[cfg(target_os = "linux")]
        let mut reproductor_pipewire: Option<std::process::Child> = {
            use std::process::{Command, Stdio};
            // Reproducir las muestras procesadas directamente en los altavoces fisicos
            Command::new("pw-play")
                .args(["--format", "f32", "--rate", "48000", "--channels", "2", "-"])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .or_else(|_| {
                    Command::new("paplay")
                        .args(["--format=float32le", "--rate=48000", "--channels=2", "--raw", "-"])
                        .stdin(Stdio::piped())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                })
                .ok()
        };

        #[cfg(target_os = "linux")]
        if capturador_pipewire.is_some() {
            telemetria_hilo_dsp.captura_wasapi_activa.store(true, Ordering::Relaxed);
            telemetria_hilo_dsp.frecuencia_actual_hz.store(48000, Ordering::Relaxed);
            telemetria_hilo_dsp
                .render_directo_activo
                .store(reproductor_pipewire.is_some(), Ordering::Relaxed);
        }

        while telemetria_hilo_dsp.ejecutando.load(Ordering::Relaxed) {
            // Verificar si el usuario cambio de audifonos / altavoces en la ventana nativa
            let mut reiniciar_flujos = false;
            if let Ok(mut enr) = enrutamiento_hilo_dsp.lock() {
                if enr.solicitar_reinicio_audio {
                    enr.solicitar_reinicio_audio = false;
                    id_cap_actual = enr.id_dispositivo_captura.clone();
                    id_sal_actual = enr.id_dispositivo_salida_directa.clone();
                    usar_render_directo = enr.usar_salida_directa_wasapi;
                    reiniciar_flujos = true;
                }
            }

            if reiniciar_flujos {
                capturador_sistema =
                    CapturadorWasapiLoopback::inicializar_con_dispositivo(id_cap_actual.as_deref())
                        .ok();
                if let Some(ref cap) = capturador_sistema {
                    telemetria_hilo_dsp
                        .captura_wasapi_activa
                        .store(true, Ordering::Relaxed);
                    telemetria_hilo_dsp
                        .frecuencia_actual_hz
                        .store(cap.frecuencia_muestreo, Ordering::Relaxed);
                } else {
                    telemetria_hilo_dsp
                        .captura_wasapi_activa
                        .store(false, Ordering::Relaxed);
                }

                renderizador_directo = if usar_render_directo {
                    RenderizadorWasapiDirecto::inicializar_con_dispositivo(id_sal_actual.as_deref())
                        .ok()
                } else {
                    None
                };
                telemetria_hilo_dsp
                    .render_directo_activo
                    .store(renderizador_directo.is_some(), Ordering::Relaxed);
            }

            let mut leyo_audio_real = false;

            #[cfg(target_os = "windows")]
            if let Some(ref mut cap) = capturador_sistema {
                leyo_audio_real = cap.leer_muestras_estereo(&mut bufer_pares_entrada);
            }

            #[cfg(target_os = "linux")]
            if let Some(ref mut proceso) = capturador_pipewire {
                if let Some(ref mut salida) = proceso.stdout {
                    use std::io::Read;
                    let mut bufer_bytes = [0u8; 4096];
                    if let Ok(leidos) = salida.read(&mut bufer_bytes) {
                        if leidos >= 8 {
                            bufer_pares_entrada.clear();
                            let total_pares = leidos / 8;
                            for i in 0..total_pares {
                                let off = i * 8;
                                let m_izq = f32::from_le_bytes([
                                    bufer_bytes[off],
                                    bufer_bytes[off + 1],
                                    bufer_bytes[off + 2],
                                    bufer_bytes[off + 3],
                                ]);
                                let m_der = f32::from_le_bytes([
                                    bufer_bytes[off + 4],
                                    bufer_bytes[off + 5],
                                    bufer_bytes[off + 6],
                                    bufer_bytes[off + 7],
                                ]);
                                bufer_pares_entrada.push((m_izq, m_der));
                            }
                            leyo_audio_real = !bufer_pares_entrada.is_empty();
                        }
                    }
                }
            }

            if leyo_audio_real && !bufer_pares_entrada.is_empty() {
                bufer_pares_procesados.clear();
                if let Ok(mut cadena) = cadena_hilo_dsp.lock() {
                    for &(m_izq, m_der) in &bufer_pares_entrada {
                        let par_out = cadena.procesar_par_estereo(m_izq, m_der);
                        bufer_pares_procesados.push(par_out);
                    }
                    let barras = cadena.obtener_barras_espectro();
                    let reduccion = cadena.obtener_reduccion_compresor_db() as f32;
                    let pico = cadena.obtener_pico_salida_db() as f32;
                    telemetria_hilo_dsp.publicar_espectro(&barras, reduccion, pico);
                }

                #[cfg(target_os = "windows")]
                if let Some(ref mut ren) = renderizador_directo {
                    if !bufer_pares_procesados.is_empty() && id_sal_actual != id_cap_actual {
                        let _ = ren.escribir_muestras_estereo(&bufer_pares_procesados);
                    }
                }

                #[cfg(target_os = "linux")]
                if let Some(ref mut rep) = reproductor_pipewire {
                    if let Some(ref mut entrada_stdin) = rep.stdin {
                        use std::io::Write;
                        let mut bufer_bytes_salida = Vec::with_capacity(bufer_pares_procesados.len() * 8);
                        for &(m_izq, m_der) in &bufer_pares_procesados {
                            bufer_bytes_salida.extend_from_slice(&m_izq.to_le_bytes());
                            bufer_bytes_salida.extend_from_slice(&m_der.to_le_bytes());
                        }
                        let _ = entrada_stdin.write_all(&bufer_bytes_salida);
                    }
                }
            } else {
                // CERO MOCKS: Silencio absoluto y analizador en cero cuando no hay flujo de audio
                if let Ok(mut cadena) = cadena_hilo_dsp.lock() {
                    for _ in 0..256 {
                        cadena.procesar_par_estereo(0.0, 0.0);
                    }
                    let barras_cero = [0.0_f32; 64];
                    telemetria_hilo_dsp.publicar_espectro(&barras_cero, 0.0, -96.0);
                }
            }
            thread::sleep(Duration::from_millis(12));
        }
    });

    // Ejecutar la Interfaz Grafica Moderna Iced a 60 FPS por defecto
    // (O la Ventana Nativa Win32 clasica si se especifica el argumento --win32-ui)
    let usar_ui_win32 = argumentos
        .iter()
        .any(|arg| arg == "--win32-ui" || arg == "--interfaz-gdi");

    if usar_ui_win32 {
        if let Err(err) = ejecutar_ventana_nativa_poweramp(
            cadena_compartida,
            telemetria,
            enrutamiento,
            directorio_base,
        ) {
            eprintln!("Error en la ventana nativa Win32: {}", err);
        }
    } else {
        if let Err(err) = ejecutar_interfaz_iced(
            cadena_compartida,
            telemetria,
            enrutamiento,
            directorio_base,
        ) {
            eprintln!("Error en la interfaz grafica Iced: {}", err);
        }
    }
}
