//! Interfaz Grafica Nativa en Rust construida con `Iced` (v0.14) y acelerada por GPU.
//! Reemplaza la ventana Win32/GDI pesada por una experiencia moderna a 60 FPS con estetica Aurora:
//! - Obsidiana Profunda (`#07090E`) como lienzo base.
//! - Ambar Sunlight (`#F59E0B`) para controles activos y curva maestra.
//! - Cherry Neon (`#FF0055`) y Cian Electrico (`#06B6D4`) para espectro FFT y dinamica.
//!
//! Organizada en 4 Escenas Soberanas:
//! 1. Escena 0: Ecualizador Parametrico (Curva Biquad + Espectro FFT 64 barras + Preamp + Tono).
//! 2. Escena 1: Espacio & Compresor Dinamico (Balance, Expansion Estereo Mid/Side, Mono y Dinamica completa sin tempo).
//! 3. Escena 2: Procesador de Reverberacion Estereo (Parametros Freeverb y Presets de Sala).
//! 4. Escena 3: Base de Datos AutoEQ (+5,400 perfiles) & Dispositivos de Audio / Estado del APO COM.
//!
//! Cumple estrictamente con la Soberania del Español y Cero Emojis.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use iced::mouse;
use iced::time;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::widget::{button, column, container, row, scrollable, slider, space, text, text_input, toggler};
use iced::{Background, Border, Color, Element, Length, Point, Rectangle, Renderer, Size, Subscription, Task, Theme};

use crate::apo_com_windows::{guardar_estado_binario_ipc, ruta_estado_ipc_global};
use crate::cadena_maestra_dsp::CadenaMaestraDsp;
use crate::captura_wasapi_loopback::{
    enumerar_dispositivos_salida_windows, DispositivoAudioWindows,
};
use crate::cargador_presets::{
    buscar_presets_autoeq_en_archivo, cargar_preset_autoeq_por_consulta, exportar_archivo_apo,
    ResumenPresetAutoEq,
};
use crate::enrutador_pipewire::{
    verificar_pipewire_disponible, EstadoEnrutadorPipeWire,
};
use crate::instalador_apo::{
    consultar_estado_instalacion_apo, desinstalar_apo, instalar_y_activar_apo,
    solicitar_elevacion_administrador, EstadoInstalacionApo,
};
use crate::tipos_audio::{
    EstadoCompletoDsp, ParametrosReverberacion,
};
use crate::ventana_nativa_win32::{EnrutamientoWasapiEstado, TelemetriaCompartidaVentana};

// Colores oficiales de la Filosofia Aurora del Aldraverso
const _COLOR_FONDO_OBSIDIANA: Color = Color::from_rgb(0.027, 0.035, 0.055); // #07090E
const _COLOR_PANEL_VIDRIO: Color = Color::from_rgba(0.07, 0.09, 0.14, 0.85);
const COLOR_AMBAR_SUNLIGHT: Color = Color::from_rgb(0.96, 0.62, 0.04); // #F59E0B
const COLOR_CHERRY_NEON: Color = Color::from_rgb(1.0, 0.0, 0.33); // #FF0055
const COLOR_CIAN_ELECTRICO: Color = Color::from_rgb(0.02, 0.71, 0.83); // #06B6D4
const COLOR_TEXTO_CLARO: Color = Color::from_rgb(0.94, 0.95, 0.97);
const COLOR_TEXTO_ATENUADO: Color = Color::from_rgb(0.55, 0.60, 0.68);
const _COLOR_BORDE_SUTIL: Color = Color::from_rgba(0.25, 0.30, 0.40, 0.4);

/// Las 5 Escenas soberanas del Ecualizador.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscenaActual {
    Ecualizador = 0,
    EspacioCompresor = 1,
    Reverberacion = 2,
    AutoEqDispositivos = 3,
    EnrutamientoPipeWire = 4,
}

/// Mensajes de eventos e interacciones de la interfaz Iced.
#[derive(Debug, Clone)]
pub enum MensajeUi {
    CambiarEscena(EscenaActual),
    TickAnimacion(std::time::Instant),
    ConmutarEcualizador(bool),
    ConmutarLimitador(bool),
    CambiarPreamp(f64),
    CambiarGananciaBanda(usize, f64),
    CambiarFrecuenciaBanda(usize, f64),
    CambiarFactorQBanda(usize, f64),
    ConmutarBanda(usize, bool),
    ConmutarTono(bool),
    CambiarTonoGraves(f64),
    CambiarTonoAgudos(f64),
    CambiarBalance(f64),
    CambiarExpansionEstereo(f64),
    ConmutarMono(bool),
    ConmutarCompresor(bool),
    CambiarCompLimite(f64),
    CambiarCompProporcion(f64),
    CambiarCompAtaque(f64),
    CambiarCompLanzamiento(f64),
    CambiarCompSidechain(f64),
    CambiarCompMaquillaje(f64),
    ConmutarReverb(bool),
    CambiarRevTamano(f64),
    CambiarRevPreretardo(f64),
    CambiarRevAmortiguacion(f64),
    CambiarRevFiltroTonal(f64),
    CambiarRevMezclado(f64),
    AplicarPresetReverb(&'static str),
    ConsultaAutoEqCambiada(String),
    EjecutarBusquedaAutoEq,
    CargarPerfilAutoEq(String),
    SeleccionarDispositivoSalida(String),
    ConmutarRenderDirectoWasapi(bool),
    InstalarApoSistema,
    DesinstalarApoSistema,
    GuardarPresetApo,
    ConmutarReproductorPipeWire(String, bool),
    ConmutarModoSelectivoPipeWire(bool),
    ActualizarListaPipeWire,
    HabilitarTodosReproductoresPipeWire,
    DeshabilitarTodosReproductoresPipeWire,
}

/// Estado global de la aplicacion Iced.
pub struct EcualizadorIcedApp {
    cadena_dsp: Arc<Mutex<CadenaMaestraDsp>>,
    telemetria: Arc<TelemetriaCompartidaVentana>,
    enrutamiento: Arc<Mutex<EnrutamientoWasapiEstado>>,
    directorio_base: PathBuf,
    escena_activa: EscenaActual,
    estado_local: EstadoCompletoDsp,
    estado_apo: EstadoInstalacionApo,
    dispositivos_audio: Vec<DispositivoAudioWindows>,
    id_dispositivo_seleccionado: String,
    usar_render_directo: bool,
    consulta_autoeq: String,
    resultados_autoeq: Vec<ResumenPresetAutoEq>,
    mensaje_notificacion: String,
    barras_espectro_suavizadas: [f32; 64],
    curva_magnitud_biquad: Vec<(f32, f32)>,
    reduccion_compresor_db: f32,
    pico_salida_db: f32,
    estado_pipewire: EstadoEnrutadorPipeWire,
    pipewire_disponible: bool,
}

impl EcualizadorIcedApp {
    pub fn nuevo(
        cadena_dsp: Arc<Mutex<CadenaMaestraDsp>>,
        telemetria: Arc<TelemetriaCompartidaVentana>,
        enrutamiento: Arc<Mutex<EnrutamientoWasapiEstado>>,
        directorio_base: PathBuf,
    ) -> Self {
        let estado_inicial = if let Ok(c) = cadena_dsp.lock() {
            c.obtener_estado().clone()
        } else {
            EstadoCompletoDsp::default()
        };

        let estado_apo = consultar_estado_instalacion_apo();
        let dispositivos_audio = enumerar_dispositivos_salida_windows().unwrap_or_default();
        let id_dispositivo_seleccionado = dispositivos_audio
            .iter()
            .find(|d| d.es_predeterminado)
            .map(|d| d.identificador.clone())
            .unwrap_or_else(|| String::from("predeterminado"));

        let mut estado_pipewire = EstadoEnrutadorPipeWire::nuevo();
        estado_pipewire.sincronizar_con_sistema();
        let pipewire_disponible = verificar_pipewire_disponible();

        let mut app = Self {
            cadena_dsp,
            telemetria,
            enrutamiento,
            directorio_base,
            escena_activa: EscenaActual::Ecualizador,
            estado_local: estado_inicial,
            estado_apo,
            dispositivos_audio,
            id_dispositivo_seleccionado,
            usar_render_directo: false,
            consulta_autoeq: String::from("JBL Tune"),
            resultados_autoeq: Vec::new(),
            mensaje_notificacion: String::from("Motor DSP listo a 64 bits"),
            barras_espectro_suavizadas: [0.0; 64],
            curva_magnitud_biquad: Vec::with_capacity(128),
            reduccion_compresor_db: 0.0,
            pico_salida_db: -96.0,
            estado_pipewire,
            pipewire_disponible,
        };

        app.recalcular_curva_magnitud();
        app.ejecutar_busqueda_autoeq_inicial();
        app
    }

    fn recalcular_curva_magnitud(&mut self) {
        let mut frecuencias = Vec::with_capacity(128);
        for i in 0..128 {
            let frac = (i as f64) / 127.0;
            let frec = 20.0 * (1000.0_f64).powf(frac); // 20 Hz a 20 kHz logaritmico
            frecuencias.push(frec);
        }

        if let Ok(cadena) = self.cadena_dsp.lock() {
            let curvas_db = cadena.evaluar_curva_respuesta_db(&frecuencias);
            self.curva_magnitud_biquad = frecuencias
                .iter()
                .zip(curvas_db.iter())
                .map(|(&f, &db)| (f as f32, db as f32))
                .collect();
        }
    }

    fn sincronizar_dsp_e_ipc(&mut self) {
        if let Ok(mut cadena) = self.cadena_dsp.lock() {
            let _ = cadena.aplicar_estado(self.estado_local.clone());
        }
        let ruta_ipc = ruta_estado_ipc_global(Some(&self.directorio_base));
        let _ = guardar_estado_binario_ipc(&ruta_ipc, &self.estado_local);
        self.recalcular_curva_magnitud();
    }

    fn ejecutar_busqueda_autoeq_inicial(&mut self) {
        let ruta_db = self.directorio_base.join("recursos").join("auto_eq.db");
        if ruta_db.exists() {
            if let Ok(res) = buscar_presets_autoeq_en_archivo(&ruta_db, &self.consulta_autoeq, 12) {
                self.resultados_autoeq = res;
            }
        }
    }

    pub fn titulo(&self) -> String {
        format!(
            "Aldra Poweramp Equalizer DSP Soberano - {} [{}]",
            self.estado_local.nombre_perfil,
            if self.estado_apo.apo_registrado {
                "APO Conectado a audiodg.exe"
            } else {
                "Modo Captura Local"
            }
        )
    }

    pub fn tema(&self) -> Theme {
        Theme::Dark
    }

    pub fn suscripcion(&self) -> Subscription<MensajeUi> {
        time::every(Duration::from_millis(16)).map(MensajeUi::TickAnimacion)
    }

    pub fn actualizar(&mut self, mensaje: MensajeUi) -> Task<MensajeUi> {
        match mensaje {
            MensajeUi::CambiarEscena(nueva) => {
                self.escena_activa = nueva;
            }
            MensajeUi::TickAnimacion(_) => {
                let (barras, red, pico) = self.telemetria.leer_espectro();
                self.reduccion_compresor_db = red;
                self.pico_salida_db = pico;
                for i in 0..64 {
                    let b = barras[i].clamp(0.0, 1.0);
                    // Suavizado balistico de caida
                    if b >= self.barras_espectro_suavizadas[i] {
                        self.barras_espectro_suavizadas[i] = b;
                    } else {
                        self.barras_espectro_suavizadas[i] =
                            self.barras_espectro_suavizadas[i] * 0.88 + b * 0.12;
                    }
                }
            }
            MensajeUi::ConmutarEcualizador(activo) => {
                self.estado_local.ecualizador_activo = activo;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::ConmutarLimitador(activo) => {
                self.estado_local.limitador_activo = activo;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::CambiarPreamp(ganancia_db) => {
                self.estado_local.ganancia_preamp_db = ganancia_db;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::CambiarGananciaBanda(indice, ganancia_db) => {
                if indice < self.estado_local.bandas.len() {
                    self.estado_local.bandas[indice].ganancia_db = ganancia_db;
                    self.sincronizar_dsp_e_ipc();
                }
            }
            MensajeUi::CambiarFrecuenciaBanda(indice, frec_hz) => {
                if indice < self.estado_local.bandas.len() {
                    self.estado_local.bandas[indice].frecuencia_hz = frec_hz;
                    self.sincronizar_dsp_e_ipc();
                }
            }
            MensajeUi::CambiarFactorQBanda(indice, factor_q) => {
                if indice < self.estado_local.bandas.len() {
                    self.estado_local.bandas[indice].factor_q = factor_q;
                    self.sincronizar_dsp_e_ipc();
                }
            }
            MensajeUi::ConmutarBanda(indice, activa) => {
                if indice < self.estado_local.bandas.len() {
                    self.estado_local.bandas[indice].activo = activa;
                    self.sincronizar_dsp_e_ipc();
                }
            }
            MensajeUi::ConmutarTono(activo) => {
                self.estado_local.tono.tono_activo = activo;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::CambiarTonoGraves(ganancia_db) => {
                self.estado_local.tono.graves_ganancia_db = ganancia_db;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::CambiarTonoAgudos(ganancia_db) => {
                self.estado_local.tono.agudos_ganancia_db = ganancia_db;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::CambiarBalance(bal) => {
                self.estado_local.espacio.balance = bal;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::CambiarExpansionEstereo(exp) => {
                self.estado_local.espacio.expansion_estereo = exp;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::ConmutarMono(mono) => {
                self.estado_local.espacio.modo_mono = mono;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::ConmutarCompresor(activo) => {
                self.estado_local.compresor.activo = activo;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::CambiarCompLimite(db) => {
                self.estado_local.compresor.limite_db = db;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::CambiarCompProporcion(ratio) => {
                self.estado_local.compresor.proporcion = ratio;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::CambiarCompAtaque(ms) => {
                self.estado_local.compresor.ataque_ms = ms;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::CambiarCompLanzamiento(ms) => {
                self.estado_local.compresor.lanzamiento_ms = ms;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::CambiarCompSidechain(hz) => {
                self.estado_local.compresor.frecuencia_sidechain_hz = hz;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::CambiarCompMaquillaje(db) => {
                self.estado_local.compresor.ganancia_maquillaje_db = db;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::ConmutarReverb(activo) => {
                self.estado_local.reverberacion.activo = activo;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::CambiarRevTamano(tamano) => {
                self.estado_local.reverberacion.tamano = tamano;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::CambiarRevPreretardo(ms) => {
                self.estado_local.reverberacion.preretardo_ms = ms;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::CambiarRevAmortiguacion(amort) => {
                self.estado_local.reverberacion.amortiguacion = amort;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::CambiarRevFiltroTonal(filtro) => {
                self.estado_local.reverberacion.filtro_tonal = filtro;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::CambiarRevMezclado(mezcla) => {
                self.estado_local.reverberacion.mezclado = mezcla;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::AplicarPresetReverb(nombre) => {
                match nombre {
                    "Estudio" => {
                        self.estado_local.reverberacion = ParametrosReverberacion {
                            activo: true,
                            tamano: 0.25,
                            preretardo_ms: 8.0,
                            amortiguacion: 0.6,
                            filtro_tonal: 0.45,
                            mezclado: 0.15,
                        };
                    }
                    "Auditorio" => {
                        self.estado_local.reverberacion = ParametrosReverberacion {
                            activo: true,
                            tamano: 0.65,
                            preretardo_ms: 22.0,
                            amortiguacion: 0.4,
                            filtro_tonal: 0.55,
                            mezclado: 0.28,
                        };
                    }
                    "Catedral" => {
                        self.estado_local.reverberacion = ParametrosReverberacion {
                            activo: true,
                            tamano: 0.92,
                            preretardo_ms: 45.0,
                            amortiguacion: 0.25,
                            filtro_tonal: 0.60,
                            mezclado: 0.40,
                        };
                    }
                    "Placa" => {
                        self.estado_local.reverberacion = ParametrosReverberacion {
                            activo: true,
                            tamano: 0.50,
                            preretardo_ms: 5.0,
                            amortiguacion: 0.15,
                            filtro_tonal: 0.80,
                            mezclado: 0.25,
                        };
                    }
                    _ => {}
                }
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::ConsultaAutoEqCambiada(texto) => {
                self.consulta_autoeq = texto;
            }
            MensajeUi::EjecutarBusquedaAutoEq => {
                let ruta_db = self.directorio_base.join("recursos").join("auto_eq.db");
                if ruta_db.exists() {
                    if let Ok(res) =
                        buscar_presets_autoeq_en_archivo(&ruta_db, &self.consulta_autoeq, 15)
                    {
                        self.resultados_autoeq = res;
                        self.mensaje_notificacion = format!(
                            "Se encontraron {} auriculares en auto_eq.db",
                            self.resultados_autoeq.len()
                        );
                    }
                }
            }
            MensajeUi::CargarPerfilAutoEq(nombre_modelo) => {
                let ruta_db = self.directorio_base.join("recursos").join("auto_eq.db");
                if ruta_db.exists() {
                    if let Ok(nuevo_estado) =
                        cargar_preset_autoeq_por_consulta(&ruta_db, &nombre_modelo)
                    {
                        self.estado_local = nuevo_estado;
                        self.sincronizar_dsp_e_ipc();
                        self.mensaje_notificacion =
                            format!("Perfil AutoEQ aplicado: {}", nombre_modelo);
                    }
                }
            }
            MensajeUi::SeleccionarDispositivoSalida(id) => {
                self.id_dispositivo_seleccionado = id.clone();
                if let Ok(mut enr) = self.enrutamiento.lock() {
                    enr.id_dispositivo_captura = Some(id.clone());
                    enr.id_dispositivo_salida_directa = Some(id);
                    enr.solicitar_reinicio_audio = true;
                }
            }
            MensajeUi::ConmutarRenderDirectoWasapi(activo) => {
                self.usar_render_directo = activo;
                if let Ok(mut enr) = self.enrutamiento.lock() {
                    enr.usar_salida_directa_wasapi = activo;
                    enr.solicitar_reinicio_audio = true;
                }
            }
            MensajeUi::InstalarApoSistema => {
                let ruta_dll = self
                    .directorio_base
                    .join("motor_rust")
                    .join("target")
                    .join("release")
                    .join("motor_ecualizador_poweramp.dll");
                match instalar_y_activar_apo(&ruta_dll) {
                    Ok(()) => {
                        self.estado_apo = consultar_estado_instalacion_apo();
                        self.mensaje_notificacion = String::from(
                            "APO instalado con exito. Reiniciando servicio de audio...",
                        );
                    }
                    Err(_) => {
                        solicitar_elevacion_administrador("--instalar-apo");
                        self.mensaje_notificacion =
                            String::from("Se solicito elevacion UAC de Administrador.");
                    }
                }
            }
            MensajeUi::DesinstalarApoSistema => {
                let _ = desinstalar_apo();
                self.estado_apo = consultar_estado_instalacion_apo();
                self.mensaje_notificacion = String::from("APO desinstalado del sistema.");
            }
            MensajeUi::GuardarPresetApo => {
                let ruta_apo = self.directorio_base.join("apo_config.txt");
                let _ = exportar_archivo_apo(&ruta_apo, &self.estado_local);
                self.mensaje_notificacion =
                    format!("Preset exportado a: {}", ruta_apo.display());
            }
            MensajeUi::ConmutarReproductorPipeWire(nombre, habilitado) => {
                self.estado_pipewire.conmutar_reproductor(&nombre, habilitado);
                self.mensaje_notificacion = format!(
                    "Reproductor '{}': {}",
                    nombre,
                    if habilitado {
                        "Ecualizacion activa (Enrutado a DSP)"
                    } else {
                        "Bypass directo (Sin procesar)"
                    }
                );
            }
            MensajeUi::ConmutarModoSelectivoPipeWire(activo) => {
                self.estado_pipewire.modo_selectivo_activo = activo;
                self.estado_pipewire.aplicar_enrutamiento_completo();
                self.mensaje_notificacion = format!(
                    "Enrutamiento PipeWire: {}",
                    if activo {
                        "Modo Selectivo (Solo aplicaciones marcadas)"
                    } else {
                        "Modo Global (Todo el audio del sistema)"
                    }
                );
            }
            MensajeUi::ActualizarListaPipeWire => {
                self.estado_pipewire.sincronizar_con_sistema();
                self.mensaje_notificacion = format!(
                    "Escaneo PipeWire: {} reproductores detectados",
                    self.estado_pipewire.reproductores.len()
                );
            }
            MensajeUi::HabilitarTodosReproductoresPipeWire => {
                for rep in &self.estado_pipewire.reproductores {
                    self.estado_pipewire.aplicaciones_habilitadas.insert(rep.nombre_aplicacion.clone());
                }
                for rep in &mut self.estado_pipewire.reproductores {
                    rep.ecualizacion_habilitada = true;
                }
                self.estado_pipewire.aplicar_enrutamiento_completo();
                self.mensaje_notificacion = String::from("Todos los reproductores fueron habilitados para DSP.");
            }
            MensajeUi::DeshabilitarTodosReproductoresPipeWire => {
                self.estado_pipewire.aplicaciones_habilitadas.clear();
                for rep in &mut self.estado_pipewire.reproductores {
                    rep.ecualizacion_habilitada = false;
                }
                self.estado_pipewire.aplicar_enrutamiento_completo();
                self.mensaje_notificacion = String::from("Todos los reproductores quedaron en Bypass directo.");
            }
        }
        Task::none()
    }

    pub fn vista(&self) -> Element<'_, MensajeUi> {
        let barra_superior = self.construir_barra_superior();
        let barra_pestanas = self.construir_barra_pestanas();

        let contenido_escena = match self.escena_activa {
            EscenaActual::Ecualizador => self.construir_escena_ecualizador(),
            EscenaActual::EspacioCompresor => self.construir_escena_espacio_compresor(),
            EscenaActual::Reverberacion => self.construir_escena_reverberacion(),
            EscenaActual::AutoEqDispositivos => self.construir_escena_autoeq_dispositivos(),
            EscenaActual::EnrutamientoPipeWire => self.construir_escena_enrutamiento_pipewire(),
        };

        let pie_telemetria = self.construir_pie_telemetria();

        container(
            column![
                barra_superior,
                barra_pestanas,
                contenido_escena,
                pie_telemetria,
            ]
            .spacing(12),
        )
        .padding(16)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn construir_barra_superior(&self) -> Element<'_, MensajeUi> {
        let titulo = text("ALDRA POWERAMP EQUALIZER")
            .size(20)
            .color(COLOR_AMBAR_SUNLIGHT);

        let perfil = text(format!("Perfil: {}", self.estado_local.nombre_perfil))
            .size(14)
            .color(COLOR_TEXTO_ATENUADO);

        let btn_bypass = toggler(
            self.estado_local.ecualizador_activo,
        )
        .label("DSP Activo")
        .on_toggle(MensajeUi::ConmutarEcualizador);

        let btn_guardar = button(text("Exportar config.txt").size(13))
            .on_press(MensajeUi::GuardarPresetApo);

        row![
            column![titulo, perfil].spacing(2),
            space::horizontal(),
            btn_bypass,
            btn_guardar,
        ]
        .spacing(16)
        .align_y(iced::Alignment::Center)
        .into()
    }

    fn construir_barra_pestanas(&self) -> Element<'_, MensajeUi> {
        let estilo_btn = |activo: bool| {
            if activo {
                COLOR_AMBAR_SUNLIGHT
            } else {
                COLOR_TEXTO_ATENUADO
            }
        };

        let btn_eq = button(
            text("1. Ecualizador & Tono")
                .size(14)
                .color(estilo_btn(self.escena_activa == EscenaActual::Ecualizador)),
        )
        .on_press(MensajeUi::CambiarEscena(EscenaActual::Ecualizador));

        let btn_esp = button(
            text("2. Espacio & Compresor")
                .size(14)
                .color(estilo_btn(self.escena_activa == EscenaActual::EspacioCompresor)),
        )
        .on_press(MensajeUi::CambiarEscena(EscenaActual::EspacioCompresor));

        let btn_rev = button(
            text("3. Reverberacion")
                .size(14)
                .color(estilo_btn(self.escena_activa == EscenaActual::Reverberacion)),
        )
        .on_press(MensajeUi::CambiarEscena(EscenaActual::Reverberacion));

        let btn_autoeq = button(
            text("4. AutoEQ & Dispositivos")
                .size(14)
                .color(estilo_btn(self.escena_activa == EscenaActual::AutoEqDispositivos)),
        )
        .on_press(MensajeUi::CambiarEscena(EscenaActual::AutoEqDispositivos));

        let etiqueta_pw = if self.pipewire_disponible {
            format!("5. Reproductores PipeWire ({})", self.estado_pipewire.reproductores.len())
        } else {
            format!("5. Reproductores Linux ({})", self.estado_pipewire.reproductores.len())
        };

        let btn_pipewire = button(
            text(etiqueta_pw)
                .size(14)
                .color(estilo_btn(self.escena_activa == EscenaActual::EnrutamientoPipeWire)),
        )
        .on_press(MensajeUi::CambiarEscena(EscenaActual::EnrutamientoPipeWire));

        row![btn_eq, btn_esp, btn_rev, btn_autoeq, btn_pipewire]
            .spacing(10)
            .into()
    }

    fn construir_escena_ecualizador(&self) -> Element<'_, MensajeUi> {
        // 1. Canvas de visualizacion grafica (Curva Biquad + Espectro FFT)
        let visualizador = Canvas::new(EspectroCurvaCanvas {
            barras_espectro: self.barras_espectro_suavizadas,
            curva_magnitud: self.curva_magnitud_biquad.clone(),
            preamp_db: self.estado_local.ganancia_preamp_db as f32,
        })
        .width(Length::Fill)
        .height(Length::Fixed(190.0));

        // 2. Faders de bandas parametricas con estetica de consola de audio (Obsidian Rack)
        let mut fila_bandas = row![].spacing(8);

        // Deslizador de Preamp Maestro
        let strip_preamp = container(
            column![
                text(format!("{:+0.1} dB", self.estado_local.ganancia_preamp_db))
                    .size(11)
                    .color(COLOR_AMBAR_SUNLIGHT),
                text("+12").size(8).color(COLOR_TEXTO_ATENUADO),
                slider(
                    -24.0..=24.0,
                    self.estado_local.ganancia_preamp_db,
                    MensajeUi::CambiarPreamp,
                )
                .step(0.1)
                .height(145.0),
                text("-24").size(8).color(COLOR_TEXTO_ATENUADO),
                button(text("0 dB").size(9).color(COLOR_TEXTO_CLARO))
                    .on_press(MensajeUi::CambiarPreamp(0.0)),
                text("PREAMP")
                    .size(10)
                    .color(COLOR_AMBAR_SUNLIGHT),
            ]
            .spacing(3)
            .align_x(iced::Alignment::Center)
        )
        .padding(6)
        .style(|_t: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgb(0.06, 0.08, 0.12))),
            border: Border {
                color: Color::from_rgba(0.96, 0.62, 0.04, 0.35),
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        });

        fila_bandas = fila_bandas.push(strip_preamp);

        for (idx, banda) in self.estado_local.bandas.iter().enumerate() {
            let etiqueta_frec = if banda.frecuencia_hz >= 1000.0 {
                format!("{:.1}k", banda.frecuencia_hz / 1000.0)
            } else {
                format!("{:.0}", banda.frecuencia_hz)
            };

            let color_ganancia = if !banda.activo {
                COLOR_TEXTO_ATENUADO
            } else if banda.ganancia_db > 0.05 {
                COLOR_AMBAR_SUNLIGHT
            } else if banda.ganancia_db < -0.05 {
                COLOR_CIAN_ELECTRICO
            } else {
                COLOR_TEXTO_CLARO
            };

            let texto_db = if banda.ganancia_db > 0.0 {
                format!("+{:.1}", banda.ganancia_db)
            } else {
                format!("{:.1}", banda.ganancia_db)
            };

            let btn_bypass = if banda.activo {
                button(text("ON").size(9).color(Color::BLACK))
                    .on_press(MensajeUi::ConmutarBanda(idx, false))
            } else {
                button(text("BYP").size(9).color(COLOR_TEXTO_ATENUADO))
                    .on_press(MensajeUi::ConmutarBanda(idx, true))
            };

            let esta_activa = banda.activo;
            let strip_banda = container(
                column![
                    text(texto_db).size(11).color(color_ganancia),
                    text("+12").size(8).color(Color::from_rgba(0.5, 0.55, 0.65, 0.3)),
                    slider(
                        -24.0..=24.0,
                        banda.ganancia_db,
                        move |val| MensajeUi::CambiarGananciaBanda(idx, val),
                    )
                    .step(0.1)
                    .height(145.0),
                    text("-24").size(8).color(Color::from_rgba(0.5, 0.55, 0.65, 0.3)),
                    btn_bypass,
                    text(etiqueta_frec)
                        .size(10)
                        .color(if esta_activa { COLOR_TEXTO_CLARO } else { COLOR_TEXTO_ATENUADO }),
                    text(format!("Q {:.1}", banda.factor_q))
                        .size(8)
                        .color(COLOR_TEXTO_ATENUADO),
                ]
                .spacing(3)
                .align_x(iced::Alignment::Center)
            )
            .padding(6)
            .style(move |_t: &Theme| container::Style {
                background: Some(Background::Color(Color::from_rgb(0.07, 0.09, 0.13))),
                border: Border {
                    color: if esta_activa {
                        Color::from_rgba(0.25, 0.30, 0.40, 0.35)
                    } else {
                        Color::from_rgba(0.15, 0.18, 0.25, 0.2)
                    },
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            });

            fila_bandas = fila_bandas.push(strip_banda);
        }

        let panel_faders = scrollable(fila_bandas)
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::default(),
            ));

        // 3. Controles de Tono (Graves y Agudos)
        let controles_tono = row![
            column![
                text("BASS / GRAVES").size(12).color(COLOR_CIAN_ELECTRICO),
                row![
                    slider(
                        -12.0..=12.0,
                        self.estado_local.tono.graves_ganancia_db,
                        MensajeUi::CambiarTonoGraves,
                    )
                    .step(0.2)
                    .width(Length::Fixed(180.0)),
                    text(format!("{:.1} dB", self.estado_local.tono.graves_ganancia_db)).size(12),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            ]
            .spacing(4),
            space::horizontal(),
            column![
                text("TREBLE / AGUDOS").size(12).color(COLOR_CIAN_ELECTRICO),
                row![
                    slider(
                        -12.0..=12.0,
                        self.estado_local.tono.agudos_ganancia_db,
                        MensajeUi::CambiarTonoAgudos,
                    )
                    .step(0.2)
                    .width(Length::Fixed(180.0)),
                    text(format!("{:.1} dB", self.estado_local.tono.agudos_ganancia_db)).size(12),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            ]
            .spacing(4),
        ]
        .padding(8);

        column![visualizador, panel_faders, controles_tono]
            .spacing(12)
            .into()
    }

    fn construir_escena_espacio_compresor(&self) -> Element<'_, MensajeUi> {
        // Panel de Espacio Estereo
        let panel_espacio = column![
            text("PROCESADOR ESPACIAL MID / SIDE")
                .size(16)
                .color(COLOR_AMBAR_SUNLIGHT),
            row![
                column![
                    text("BALANCE L / R").size(12).color(COLOR_TEXTO_ATENUADO),
                    slider(
                        -1.0..=1.0,
                        self.estado_local.espacio.balance,
                        MensajeUi::CambiarBalance,
                    )
                    .step(0.01)
                    .width(Length::Fixed(220.0)),
                    text(format!("{:+.2}", self.estado_local.espacio.balance)).size(12),
                ]
                .spacing(4),
                space::horizontal(),
                column![
                    text("EXPANSION ESTEREO (ANCHO)").size(12).color(COLOR_CIAN_ELECTRICO),
                    slider(
                        0.0..=1.5,
                        self.estado_local.espacio.expansion_estereo,
                        MensajeUi::CambiarExpansionEstereo,
                    )
                    .step(0.02)
                    .width(Length::Fixed(220.0)),
                    text(format!(
                        "{:.0} %",
                        self.estado_local.espacio.expansion_estereo * 100.0
                    ))
                    .size(12),
                ]
                .spacing(4),
                space::horizontal(),
                toggler(
                    self.estado_local.espacio.modo_mono,
                )
                .label("Modo Mono")
                .on_toggle(MensajeUi::ConmutarMono),
            ]
            .spacing(16)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(8);

        // Panel de Compresor Dinamico
        let comp = self.estado_local.compresor;
        let panel_compresor = column![
            row![
                text("COMPRESOR DINAMICO ESTEREO")
                    .size(16)
                    .color(COLOR_CHERRY_NEON),
                space::horizontal(),
                toggler(comp.activo)
                    .label("Compresor Activo")
                    .on_toggle(MensajeUi::ConmutarCompresor),
            ]
            .align_y(iced::Alignment::Center),
            text(format!(
                "Reduccion de Ganancia Activa: -{:.1} dB",
                self.reduccion_compresor_db
            ))
            .size(13)
            .color(COLOR_CHERRY_NEON),
            row![
                column![
                    text("Umbral (Threshold)").size(11),
                    slider(-60.0..=0.0, comp.limite_db, MensajeUi::CambiarCompLimite)
                        .step(0.5)
                        .width(Length::Fixed(180.0)),
                    text(format!("{:.1} dB", comp.limite_db)).size(11),
                ]
                .spacing(3),
                column![
                    text("Proporcion (Ratio)").size(11),
                    slider(1.0..=20.0, comp.proporcion, MensajeUi::CambiarCompProporcion)
                        .step(0.1)
                        .width(Length::Fixed(180.0)),
                    text(format!("{:.1}:1", comp.proporcion)).size(11),
                ]
                .spacing(3),
                column![
                    text("Ataque (Attack)").size(11),
                    slider(0.1..=150.0, comp.ataque_ms, MensajeUi::CambiarCompAtaque)
                        .step(0.5)
                        .width(Length::Fixed(180.0)),
                    text(format!("{:.1} ms", comp.ataque_ms)).size(11),
                ]
                .spacing(3),
            ]
            .spacing(16),
            row![
                column![
                    text("Lanzamiento (Release)").size(11),
                    slider(
                        10.0..=1000.0,
                        comp.lanzamiento_ms,
                        MensajeUi::CambiarCompLanzamiento,
                    )
                    .step(5.0)
                    .width(Length::Fixed(180.0)),
                    text(format!("{:.0} ms", comp.lanzamiento_ms)).size(11),
                ]
                .spacing(3),
                column![
                    text("Filtro Sidechain HPF").size(11),
                    slider(
                        20.0..=500.0,
                        comp.frecuencia_sidechain_hz,
                        MensajeUi::CambiarCompSidechain,
                    )
                    .step(5.0)
                    .width(Length::Fixed(180.0)),
                    text(format!("{:.0} Hz", comp.frecuencia_sidechain_hz)).size(11),
                ]
                .spacing(3),
                column![
                    text("Maquillaje (Makeup Gain)").size(11),
                    slider(
                        0.0..=24.0,
                        comp.ganancia_maquillaje_db,
                        MensajeUi::CambiarCompMaquillaje,
                    )
                    .step(0.2)
                    .width(Length::Fixed(180.0)),
                    text(format!("{:.1} dB", comp.ganancia_maquillaje_db)).size(11),
                ]
                .spacing(3),
            ]
            .spacing(16),
        ]
        .spacing(10);

        column![panel_espacio, space::vertical(), panel_compresor]
            .spacing(20)
            .into()
    }

    fn construir_escena_reverberacion(&self) -> Element<'_, MensajeUi> {
        let rev = self.estado_local.reverberacion;

        let presets_rapidos = row![
            button(text("Estudio").size(12)).on_press(MensajeUi::AplicarPresetReverb("Estudio")),
            button(text("Auditorio").size(12)).on_press(MensajeUi::AplicarPresetReverb("Auditorio")),
            button(text("Catedral").size(12)).on_press(MensajeUi::AplicarPresetReverb("Catedral")),
            button(text("Placa Metalica").size(12)).on_press(MensajeUi::AplicarPresetReverb("Placa")),
        ]
        .spacing(8);

        column![
            row![
                text("PROCESADOR DE REVERBERACION SCHROEDER-MOORER")
                    .size(16)
                    .color(COLOR_AMBAR_SUNLIGHT),
                space::horizontal(),
                toggler(rev.activo)
                    .label("Reverb Activa")
                    .on_toggle(MensajeUi::ConmutarReverb),
            ]
            .align_y(iced::Alignment::Center),
            row![text("Presets recomendados:").size(12).color(COLOR_TEXTO_ATENUADO), presets_rapidos]
                .spacing(12)
                .align_y(iced::Alignment::Center),
            row![
                column![
                    text("Tamano de Sala").size(11),
                    slider(0.0..=1.0, rev.tamano, MensajeUi::CambiarRevTamano)
                        .step(0.01)
                        .width(Length::Fixed(220.0)),
                    text(format!("{:.0} %", rev.tamano * 100.0)).size(11),
                ]
                .spacing(3),
                column![
                    text("Preretardo (Pre-delay)").size(11),
                    slider(0.0..=100.0, rev.preretardo_ms, MensajeUi::CambiarRevPreretardo)
                        .step(1.0)
                        .width(Length::Fixed(220.0)),
                    text(format!("{:.0} ms", rev.preretardo_ms)).size(11),
                ]
                .spacing(3),
            ]
            .spacing(20),
            row![
                column![
                    text("Amortiguacion Agudos").size(11),
                    slider(0.0..=1.0, rev.amortiguacion, MensajeUi::CambiarRevAmortiguacion)
                        .step(0.01)
                        .width(Length::Fixed(220.0)),
                    text(format!("{:.0} %", rev.amortiguacion * 100.0)).size(11),
                ]
                .spacing(3),
                column![
                    text("Filtro Tonal (Calido / Brillante)").size(11),
                    slider(0.0..=1.0, rev.filtro_tonal, MensajeUi::CambiarRevFiltroTonal)
                        .step(0.01)
                        .width(Length::Fixed(220.0)),
                    text(format!("{:.2}", rev.filtro_tonal)).size(11),
                ]
                .spacing(3),
                column![
                    text("Mezclado Dry / Wet").size(11).color(COLOR_CIAN_ELECTRICO),
                    slider(0.0..=1.0, rev.mezclado, MensajeUi::CambiarRevMezclado)
                        .step(0.01)
                        .width(Length::Fixed(220.0)),
                    text(format!("{:.0} %", rev.mezclado * 100.0)).size(11),
                ]
                .spacing(3),
            ]
            .spacing(20),
        ]
        .spacing(16)
        .into()
    }

    fn construir_escena_autoeq_dispositivos(&self) -> Element<'_, MensajeUi> {
        // Seccion AutoEQ
        let entrada_busqueda = text_input(
            "Buscar modelo de auriculares (ej. Sony WH-1000XM4, HD650, JBL, AirPods)...",
            &self.consulta_autoeq,
        )
        .on_input(MensajeUi::ConsultaAutoEqCambiada)
        .on_submit(MensajeUi::EjecutarBusquedaAutoEq)
        .padding(8);

        let btn_buscar = button(text("Buscar en DB").size(13))
            .on_press(MensajeUi::EjecutarBusquedaAutoEq);

        let mut lista_auriculares = column![].spacing(6);
        for item in &self.resultados_autoeq {
            let nombre = item.nombre_modelo.clone();
            let subtexto = format!(
                "{} ({} bandas {})",
                item.nombre_modelo,
                item.numero_bandas,
                if item.es_parametrico {
                    "Parametrico"
                } else {
                    "Grafico"
                }
            );

            let fila = row![
                text(subtexto).size(12).width(Length::Fill),
                button(text("Aplicar").size(11))
                    .on_press(MensajeUi::CargarPerfilAutoEq(nombre)),
            ]
            .align_y(iced::Alignment::Center)
            .padding(4);

            lista_auriculares = lista_auriculares.push(fila);
        }

        let panel_autoeq = column![
            text("BASE DE DATOS AUTOEQ (+5,400 MODELOS)")
                .size(15)
                .color(COLOR_AMBAR_SUNLIGHT),
            row![entrada_busqueda, btn_buscar].spacing(8),
            scrollable(lista_auriculares).height(Length::Fixed(150.0)),
        ]
        .spacing(8);

        // Seccion APO e Inyeccion de Audio en Windows
        let estado_apo_txt = if self.estado_apo.apo_registrado {
            text("ESTADO: APO ACTIVO en audiodg.exe (El sonido de TODO Windows es procesado)")
                .size(13)
                .color(COLOR_AMBAR_SUNLIGHT)
        } else {
            text("ESTADO: APO NO REGISTRADO (Solo modo visualizador / render local)")
                .size(13)
                .color(COLOR_CHERRY_NEON)
        };

        let btn_instalar = if self.estado_apo.apo_registrado {
            button(text("Desinstalar APO del Sistema").size(12))
                .on_press(MensajeUi::DesinstalarApoSistema)
        } else {
            button(text("Instalar y Activar APO en Windows").size(12))
                .on_press(MensajeUi::InstalarApoSistema)
        };

        let conmutador_directo = toggler(
            self.usar_render_directo,
        )
        .label("Renderizado Directo WASAPI de Salida (Modo Inmediato sin reiniciar audiosrv)")
        .on_toggle(MensajeUi::ConmutarRenderDirectoWasapi);

        let mut lista_dispositivos = column![].spacing(4);
        for dev in &self.dispositivos_audio {
            let id = dev.identificador.clone();
            let es_seleccionado = dev.identificador == self.id_dispositivo_seleccionado;
            let etiqueta = if dev.es_predeterminado {
                format!("[Predeterminado] {}", dev.nombre_amigable)
            } else {
                dev.nombre_amigable.clone()
            };
            let fila = row![
                text(etiqueta)
                    .size(12)
                    .color(if es_seleccionado {
                        COLOR_AMBAR_SUNLIGHT
                    } else {
                        COLOR_TEXTO_CLARO
                    })
                    .width(Length::Fill),
                button(text(if es_seleccionado { "Activo" } else { "Seleccionar" }).size(11))
                    .on_press(MensajeUi::SeleccionarDispositivoSalida(id)),
            ]
            .align_y(iced::Alignment::Center)
            .padding(2);
            lista_dispositivos = lista_dispositivos.push(fila);
        }

        let panel_dispositivos = column![
            text("DISPOSITIVOS DE AUDIO DETECTADOS (WASAPI)")
                .size(13)
                .color(COLOR_TEXTO_ATENUADO),
            scrollable(lista_dispositivos).height(Length::Fixed(90.0)),
        ]
        .spacing(4);

        let panel_sistema = column![
            text("CONTROL DEL APO DE WINDOWS & DISPOSITIVOS")
                .size(15)
                .color(COLOR_CIAN_ELECTRICO),
            estado_apo_txt,
            row![btn_instalar, conmutador_directo].spacing(16),
            panel_dispositivos,
        ]
        .spacing(8);

        column![panel_autoeq, space::vertical(), panel_sistema]
            .spacing(16)
            .into()
    }

    fn construir_escena_enrutamiento_pipewire(&self) -> Element<'_, MensajeUi> {
        let titulo = text("ENRUTADOR DE STREAMS PIPEWIRE (LINUX)")
            .size(18)
            .color(COLOR_AMBAR_SUNLIGHT);

        let descripcion = text(
            "Identificacion en tiempo real de reproductores emitiendo audio en el sistema. \
             Permite seleccionar que aplicaciones pasan por la cadena de ecualizacion Poweramp y cuales permanecen en bypass directo."
        )
        .size(12)
        .color(COLOR_TEXTO_ATENUADO);

        // Barra de control de la escena PipeWire
        let estado_badge = if self.pipewire_disponible {
            text("● Servidor PipeWire Activo")
                .size(12)
                .color(Color::from_rgb(0.06, 0.72, 0.51))
        } else {
            text("○ Modo Simulacion Multiplataforma")
                .size(12)
                .color(COLOR_CIAN_ELECTRICO)
        };

        let switch_selectivo = toggler(self.estado_pipewire.modo_selectivo_activo)
            .label("Modo Selectivo (Solo seleccionados)")
            .on_toggle(MensajeUi::ConmutarModoSelectivoPipeWire);

        let btn_refrescar = button(text("Escanear Flujos").size(12))
            .on_press(MensajeUi::ActualizarListaPipeWire);

        let btn_todos = button(text("Ecualizar Todos").size(12))
            .on_press(MensajeUi::HabilitarTodosReproductoresPipeWire);

        let btn_ninguno = button(text("Bypass General").size(12))
            .on_press(MensajeUi::DeshabilitarTodosReproductoresPipeWire);

        let barra_control = container(
            row![
                column![estado_badge, switch_selectivo].spacing(4),
                space::horizontal(),
                btn_refrescar,
                btn_todos,
                btn_ninguno,
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center),
        )
        .padding(12)
        .style(|_t: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgb(0.06, 0.08, 0.12))),
            border: Border {
                color: Color::from_rgba(0.25, 0.30, 0.40, 0.4),
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        });

        // Contenido de la lista de reproductores
        let mut columna_reproductores = column![].spacing(8);

        if self.estado_pipewire.reproductores.is_empty() {
            let aviso_vacio = container(
                column![
                    text("No se detectaron flujos de audio activos en PipeWire.")
                        .size(15)
                        .color(COLOR_AMBAR_SUNLIGHT),
                    text("Inicia la reproduccion de audio en Spotify, Firefox, VLC o tu navegador y presiona 'Escanear Flujos'.")
                        .size(12)
                        .color(COLOR_TEXTO_ATENUADO),
                ]
                .spacing(8)
                .align_x(iced::Alignment::Center),
            )
            .padding(32)
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center);

            columna_reproductores = columna_reproductores.push(aviso_vacio);
        } else {
            for rep in &self.estado_pipewire.reproductores {
                let nombre_app = rep.nombre_aplicacion.clone();
                let esta_ecualizado = rep.ecualizacion_habilitada;

                let estado_reproduccion = if rep.esta_reproduciendo {
                    text("● REPRODUCIENDO SONIDO EN VIVO")
                        .size(10)
                        .color(Color::from_rgb(0.06, 0.72, 0.51))
                } else {
                    text("○ Inactivo / En Pausa")
                        .size(10)
                        .color(COLOR_TEXTO_ATENUADO)
                };

                let subtexto = if !rep.nombre_medio.is_empty() {
                    format!("{} | Nodo #{}", rep.nombre_medio, rep.identificador_nodo)
                } else {
                    format!("Stream de audio | Nodo #{}", rep.identificador_nodo)
                };

                let col_info = column![
                    row![
                        text(&rep.nombre_aplicacion).size(16).color(COLOR_TEXTO_CLARO),
                        space::horizontal(),
                        estado_reproduccion,
                    ]
                    .align_y(iced::Alignment::Center),
                    text(subtexto).size(11).color(COLOR_TEXTO_ATENUADO),
                ]
                .spacing(4)
                .width(Length::Fill);

                let switch_rep = toggler(esta_ecualizado)
                    .label(if esta_ecualizado {
                        "ECUALIZANDO CON DSP"
                    } else {
                        "PASO DIRECTO (BYPASS)"
                    })
                    .on_toggle(move |activo| {
                        MensajeUi::ConmutarReproductorPipeWire(nombre_app.clone(), activo)
                    });

                let tarjeta = container(
                    row![col_info, switch_rep]
                        .spacing(16)
                        .align_y(iced::Alignment::Center),
                )
                .padding(14)
                .style(move |_t: &Theme| container::Style {
                    background: Some(Background::Color(if esta_ecualizado {
                        Color::from_rgb(0.08, 0.10, 0.15)
                    } else {
                        Color::from_rgb(0.05, 0.06, 0.09)
                    })),
                    border: Border {
                        color: if esta_ecualizado {
                            Color::from_rgba(0.96, 0.62, 0.04, 0.45)
                        } else {
                            Color::from_rgba(0.20, 0.25, 0.35, 0.25)
                        },
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                });

                columna_reproductores = columna_reproductores.push(tarjeta);
            }
        }

        let lista_scroll = scrollable(columna_reproductores).height(Length::Fill);

        column![
            titulo,
            descripcion,
            barra_control,
            lista_scroll,
        ]
        .spacing(12)
        .into()
    }

    fn construir_pie_telemetria(&self) -> Element<'_, MensajeUi> {
        let texto_notif = text(&self.mensaje_notificacion)
            .size(11)
            .color(COLOR_TEXTO_ATENUADO);

        let medidor_pico = text(format!("Pico Salida: {:.1} dBFS", self.pico_salida_db))
            .size(11)
            .color(if self.pico_salida_db > -0.5 {
                COLOR_CHERRY_NEON
            } else {
                COLOR_AMBAR_SUNLIGHT
            });

        row![texto_notif, space::horizontal(), medidor_pico]
            .align_y(iced::Alignment::Center)
            .into()
    }
}

/// Canvas para dibujar la Curva de Magnitud Biquad y las 64 barras FFT en tiempo real.
struct EspectroCurvaCanvas {
    barras_espectro: [f32; 64],
    curva_magnitud: Vec<(f32, f32)>,
    preamp_db: f32,
}

impl<Mensaje> canvas::Program<Mensaje> for EspectroCurvaCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let w = bounds.width;
        let h = bounds.height;

        // Fondo oscuro de rejilla
        frame.fill_rectangle(
            Point::ORIGIN,
            Size::new(w, h),
            Color::from_rgba(0.04, 0.05, 0.08, 0.95),
        );

        // Grilla de lineas horizontales (dB)
        let niveles_db = [-18.0, -12.0, -6.0, 0.0, 6.0, 12.0, 18.0];
        for &db in &niveles_db {
            let y = h * (0.5 - (db / 48.0));
            let color_linea = if db.abs() < 0.1 {
                Color::from_rgba(0.96, 0.62, 0.04, 0.35) // 0 dB dorado
            } else {
                Color::from_rgba(0.2, 0.25, 0.35, 0.18)
            };

            let path_linea = Path::line(Point::new(0.0, y), Point::new(w, y));
            frame.stroke(
                &path_linea,
                Stroke::default().with_width(1.0).with_color(color_linea),
            );
        }

        // Linea de referencia dorada para el Preamplificador
        if self.preamp_db.abs() > 0.05 {
            let y_pre = (h * (0.5 - (self.preamp_db / 48.0))).clamp(2.0, h - 2.0);
            let path_pre = Path::line(Point::new(0.0, y_pre), Point::new(w, y_pre));
            frame.stroke(
                &path_pre,
                Stroke::default()
                    .with_width(1.0)
                    .with_color(Color::from_rgba(0.96, 0.62, 0.04, 0.45)),
            );
        }

        // Dibujar las 64 barras de espectro FFT
        let num_barras = self.barras_espectro.len();
        let paso_x = w / (num_barras as f32);
        let ancho_barra = (paso_x * 0.75).max(1.5);

        for (i, &amplitud) in self.barras_espectro.iter().enumerate() {
            let alt_barra = (amplitud.clamp(0.0, 1.0) * (h * 0.85)).max(2.0);
            let x = (i as f32) * paso_x + (paso_x - ancho_barra) * 0.5;
            let y = h - alt_barra;

            // Degradado visual aurora segun energia
            let color_barra = if amplitud > 0.75 {
                COLOR_CHERRY_NEON
            } else if amplitud > 0.4 {
                COLOR_AMBAR_SUNLIGHT
            } else {
                Color::from_rgba(0.02, 0.71, 0.83, 0.65)
            };

            frame.fill_rectangle(
                Point::new(x, y),
                Size::new(ancho_barra, alt_barra),
                color_barra,
            );
        }

        // Dibujar la curva continua de respuesta en frecuencia Biquad
        if self.curva_magnitud.len() >= 2 {
            let mut builder = canvas::path::Builder::new();
            let mut primer_punto = true;

            for &(frec, ganancia_db) in &self.curva_magnitud {
                let norm_x = ((frec / 20.0).log10() / (1000.0_f32).log10()).clamp(0.0, 1.0);
                let x = norm_x * w;
                let norm_y = (0.5 - (ganancia_db / 48.0)).clamp(0.05, 0.95);
                let y = norm_y * h;

                let pt = Point::new(x, y);
                if primer_punto {
                    builder.move_to(pt);
                    primer_punto = false;
                } else {
                    builder.line_to(pt);
                }
            }

            let path_curva = builder.build();
            frame.stroke(
                &path_curva,
                Stroke::default()
                    .with_width(2.5)
                    .with_color(COLOR_AMBAR_SUNLIGHT),
            );
        }

        vec![frame.into_geometry()]
    }
}

/// Punto de entrada que lanza la aplicacion nativa Iced para el Ecualizador.
pub fn ejecutar_interfaz_iced(
    cadena_dsp: Arc<Mutex<CadenaMaestraDsp>>,
    telemetria: Arc<TelemetriaCompartidaVentana>,
    enrutamiento: Arc<Mutex<EnrutamientoWasapiEstado>>,
    directorio_base: PathBuf,
) -> Result<(), iced::Error> {
    let cadena_clon = cadena_dsp.clone();
    let telemetria_clon = telemetria.clone();
    let enrutamiento_clon = enrutamiento.clone();
    let base_clon = directorio_base.clone();

    iced::application(
        move || {
            EcualizadorIcedApp::nuevo(
                cadena_clon.clone(),
                telemetria_clon.clone(),
                enrutamiento_clon.clone(),
                base_clon.clone(),
            )
        },
        EcualizadorIcedApp::actualizar,
        EcualizadorIcedApp::vista,
    )
    .title(EcualizadorIcedApp::titulo)
    .theme(EcualizadorIcedApp::tema)
    .subscription(EcualizadorIcedApp::suscripcion)
    .window(iced::window::Settings {
        size: Size::new(1024.0, 720.0),
        min_size: Some(Size::new(860.0, 580.0)),
        resizable: true,
        decorations: true,
        transparent: false,
        ..Default::default()
    })
    .run()
}
