//! Ejecutable principal del Ecualizador de Escritorio Soberano en Rust (`motor_dsp_poweramp.exe`).
//! Provee de forma 100% nativa e integrada:
//! 1. Ventana Nativa de Escritorio Win32/GDI con Doble Bufer a 60 FPS (`ventana_nativa_win32.rs`)
//!    reconstruida a partir de los layouts AXML extraidos con REA de `Equalizador Poweramp.apk`.
//! 2. Hilo de Audio DSP en Tiempo Real (64-bit Biquad + Tono + Espacio Mid/Side + Compresor + Reverb + FFT 64 barras)
//!    con Captura WASAPI Loopback (`CapturadorWasapiLoopback`) y Renderizado Directo WASAPI (`RenderizadorWasapiDirecto`).
//! 3. Sincronizacion binaria IPC (`estado_dsp_activo.bin`) con nuestro propio Objeto COM APO de Windows (`apo_com_windows.rs`).

use std::env;
use std::f32::consts::PI;
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
    buscar_presets_autoeq_en_archivo, cargar_preset_json_desde_archivo, exportar_archivo_apo,
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
    let argumentos: Vec<String> = env::args().collect();
    let directorio_base = localizar_directorio_base();

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

    // Inicializar Cadena Maestra DSP a 48 kHz
    let mut cadena_inicial = CadenaMaestraDsp::nueva(48000.0);

    // Si existe el preset JBL Tune 720BT en el directorio base, cargarlo inicialmente
    let ruta_jbl =
        directorio_base.join("JBL Tune 720BT (passive) [28 jul 2026 8_55_44\u{202f}p.m.].json");
    if ruta_jbl.exists() {
        if let Ok(estado_jbl) = cargar_preset_json_desde_archivo(&ruta_jbl) {
            let _ = cadena_inicial.aplicar_estado(estado_jbl);
        }
    }

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
        let mut fase_1 = 0.0_f32;
        let mut fase_2 = 0.0_f32;
        let mut fase_3 = 0.0_f32;
        let mut fase_4 = 0.0_f32;
        let mut modulador_lento = 0.0_f32;

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

            if let Some(ref mut cap) = capturador_sistema {
                let leyo_paquetes = cap.leer_muestras_estereo(&mut bufer_pares_entrada);
                bufer_pares_procesados.clear();

                if let Ok(mut cadena) = cadena_hilo_dsp.lock() {
                    if leyo_paquetes && !bufer_pares_entrada.is_empty() {
                        for &(m_izq, m_der) in &bufer_pares_entrada {
                            let par_out = cadena.procesar_par_estereo(m_izq, m_der);
                            bufer_pares_procesados.push(par_out);
                        }
                    } else {
                        for _ in 0..512 {
                            cadena.procesar_par_estereo(0.0, 0.0);
                        }
                    }

                    let barras = cadena.obtener_barras_espectro();
                    let reduccion = cadena.obtener_reduccion_compresor_db() as f32;
                    let pico = cadena.obtener_pico_salida_db() as f32;
                    telemetria_hilo_dsp.publicar_espectro(&barras, reduccion, pico);
                }

                // Si el Enrutador Directo WASAPI esta activo hacia un dispositivo distinto al de captura
                // (o cuando el usuario desea inyectar la diferencia), enviar las muestras procesadas
                if let Some(ref mut ren) = renderizador_directo {
                    if !bufer_pares_procesados.is_empty()
                        && id_sal_actual != id_cap_actual
                    {
                        let _ = ren.escribir_muestras_estereo(&bufer_pares_procesados);
                    }
                }
            } else if let Ok(mut cadena) = cadena_hilo_dsp.lock() {
                let fs = 48000.0_f32;
                for _ in 0..512 {
                    modulador_lento += 1.0 / fs;
                    let envolvente_ritmica = 0.35 + 0.25 * (modulador_lento * 4.2).sin().abs();

                    fase_1 = (fase_1 + 2.0 * PI * 65.0 / fs).rem_euclid(2.0 * PI);
                    fase_2 = (fase_2 + 2.0 * PI * 220.0 / fs).rem_euclid(2.0 * PI);
                    fase_3 = (fase_3 + 2.0 * PI * 1250.0 / fs).rem_euclid(2.0 * PI);
                    fase_4 = (fase_4 + 2.0 * PI * 6400.0 / fs).rem_euclid(2.0 * PI);

                    let m_izq = envolvente_ritmica
                        * (0.45 * fase_1.sin()
                            + 0.25 * fase_2.sin()
                            + 0.20 * fase_3.sin()
                            + 0.10 * fase_4.sin());
                    let m_der = envolvente_ritmica
                        * (0.45 * fase_1.cos()
                            + 0.25 * fase_2.cos()
                            + 0.20 * fase_3.sin()
                            + 0.10 * fase_4.cos());

                    cadena.procesar_par_estereo(m_izq, m_der);
                }

                let barras = cadena.obtener_barras_espectro();
                let reduccion = cadena.obtener_reduccion_compresor_db() as f32;
                let pico = cadena.obtener_pico_salida_db() as f32;
                telemetria_hilo_dsp.publicar_espectro(&barras, reduccion, pico);
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
