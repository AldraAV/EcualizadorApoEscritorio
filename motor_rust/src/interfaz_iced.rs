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
    BandaParametrica, EstadoCompletoDsp, ParametrosReverberacion, TipoFiltro,
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

/// Definicion de un preset grafico estandar de 10 bandas ISO.
#[derive(Debug, Clone, Copy)]
pub struct PresetGraficoDef {
    pub nombre: &'static str,
    pub ganancias_db: [f64; 10],
    pub preamp_db: f64,
    pub descripcion: &'static str,
}

/// Catalogo oficial de los 23 presets graficos de fabrica de Poweramp.
pub const PRESETS_POWERAMP_GRAFICOS: [PresetGraficoDef; 23] = [
    PresetGraficoDef {
        nombre: "Plano (0 dB)",
        ganancias_db: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        preamp_db: 0.0,
        descripcion: "Respuesta plana neutra de referencia",
    },
    PresetGraficoDef {
        nombre: "Bass Booster",
        ganancias_db: [7.5, 6.0, 4.5, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        preamp_db: -4.0,
        descripcion: "Realce calido de frecuencias sub-graves y bajas",
    },
    PresetGraficoDef {
        nombre: "Bass Heavy",
        ganancias_db: [9.0, 7.5, 5.5, 3.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        preamp_db: -5.0,
        descripcion: "Graves masivos y profundos para bass music",
    },
    PresetGraficoDef {
        nombre: "Punchy Bass",
        ganancias_db: [4.0, 8.0, 6.0, 2.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        preamp_db: -4.0,
        descripcion: "Pegada de bombo seco y definicion ritmica",
    },
    PresetGraficoDef {
        nombre: "Soft Bass",
        ganancias_db: [4.5, 3.5, 2.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        preamp_db: -2.0,
        descripcion: "Calidez suave y redondeada sin distorsion",
    },
    PresetGraficoDef {
        nombre: "Bass & Treble",
        ganancias_db: [6.0, 4.5, 2.0, 0.0, -1.0, 0.0, 1.5, 3.5, 5.5, 6.5],
        preamp_db: -3.5,
        descripcion: "Curva clasica en V para realzar extremos",
    },
    PresetGraficoDef {
        nombre: "Treble",
        ganancias_db: [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 2.5, 4.5, 6.5, 7.5],
        preamp_db: -4.0,
        descripcion: "Claridad y presencia en frecuencias altas",
    },
    PresetGraficoDef {
        nombre: "Treble Booster",
        ganancias_db: [0.0, 0.0, 0.0, 0.0, 0.0, 1.5, 3.5, 6.0, 8.5, 10.0],
        preamp_db: -5.0,
        descripcion: "Maximo realce para platillos y detalles de aire",
    },
    PresetGraficoDef {
        nombre: "Soft Treble",
        ganancias_db: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 3.5],
        preamp_db: -2.0,
        descripcion: "Apertura suave y relajada sin fatiga",
    },
    PresetGraficoDef {
        nombre: "Rock",
        ganancias_db: [4.8, 3.2, -1.5, -2.8, -1.0, 1.5, 3.0, 4.0, 4.5, 5.0],
        preamp_db: -2.5,
        descripcion: "Bombo con cuerpo, medios limpios y guitarras nitidas",
    },
    PresetGraficoDef {
        nombre: "Pop",
        ganancias_db: [-1.5, 1.5, 3.5, 4.5, 3.0, -1.0, -1.5, -1.0, 2.0, 3.5],
        preamp_db: -2.0,
        descripcion: "Voces cristalinas al frente y ritmo compensado",
    },
    PresetGraficoDef {
        nombre: "Techno",
        ganancias_db: [5.5, 4.5, 1.0, 0.0, -2.0, 2.0, 1.0, 3.5, 5.0, 5.5],
        preamp_db: -3.0,
        descripcion: "Lineas de bajo potentes y sintetizadores cortantes",
    },
    PresetGraficoDef {
        nombre: "Dance",
        ganancias_db: [6.0, 5.0, 2.0, 0.0, -1.5, 1.5, 2.5, 4.0, 5.0, 5.5],
        preamp_db: -3.5,
        descripcion: "Sonido de club dinamico y envolvente",
    },
    PresetGraficoDef {
        nombre: "Clasica",
        ganancias_db: [4.5, 3.5, 2.5, 2.0, -1.5, -1.5, 0.0, 2.5, 3.5, 4.0],
        preamp_db: -2.0,
        descripcion: "Amplitud acustica orquestal y cuerdas calidas",
    },
    PresetGraficoDef {
        nombre: "Live / En Vivo",
        ganancias_db: [-2.0, 0.0, 2.0, 3.0, 3.5, 3.5, 3.0, 2.0, 1.5, 1.0],
        preamp_db: -2.0,
        descripcion: "Espacialidad y sensacion de sala de concierto",
    },
    PresetGraficoDef {
        nombre: "Party / Fiesta",
        ganancias_db: [5.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 4.0, 5.0],
        preamp_db: -3.0,
        descripcion: "Volumen percibido elevado y extremos reforzados",
    },
    PresetGraficoDef {
        nombre: "Ska",
        ganancias_db: [-1.5, -2.5, -1.5, 0.0, 2.5, 3.5, 4.0, 4.5, 5.0, 4.0],
        preamp_db: -2.5,
        descripcion: "Metales agudos y ritmo en contratiempo destacado",
    },
    PresetGraficoDef {
        nombre: "Soft / Suave",
        ganancias_db: [3.0, 1.5, 0.0, -1.0, -1.0, 0.0, 1.0, 2.0, 3.0, 3.5],
        preamp_db: -2.0,
        descripcion: "Atenuacion de asperezas para escucha prolongada",
    },
    PresetGraficoDef {
        nombre: "Reggae",
        ganancias_db: [0.0, 0.0, -0.5, -3.0, 0.0, 4.0, 4.5, 3.0, 0.0, 0.0],
        preamp_db: -2.5,
        descripcion: "Bajo redondo y caja ritmica acentuada",
    },
    PresetGraficoDef {
        nombre: "Vocal / Voz Clara",
        ganancias_db: [-3.5, -2.5, 0.0, 2.5, 4.0, 4.5, 3.5, 1.5, 0.0, -2.0],
        preamp_db: -2.0,
        descripcion: "Maxima inteligibilidad vocal para podcast y dialogos",
    },
    PresetGraficoDef {
        nombre: "Acustico",
        ganancias_db: [3.5, 2.5, 1.5, 1.0, 1.5, 2.0, 3.0, 3.5, 3.0, 2.0],
        preamp_db: -2.0,
        descripcion: "Guitarras de madera, piano y percusion organica",
    },
    PresetGraficoDef {
        nombre: "Electronica",
        ganancias_db: [6.5, 5.5, 2.0, 0.0, -1.5, 2.5, 1.5, 3.5, 5.5, 6.0],
        preamp_db: -3.5,
        descripcion: "Sintetizadores analogicos y sub-frecuencias profundas",
    },
    PresetGraficoDef {
        nombre: "Harman Target",
        ganancias_db: [5.5, 4.0, 1.5, -1.0, -0.5, 0.0, 1.0, 3.5, 1.0, 0.0],
        preamp_db: -2.5,
        descripcion: "Curva objetivo psicoacustica Harman de alta fidelidad",
    },
];

/// Definicion de una banda para un preset parametrico.
#[derive(Debug, Clone, Copy)]
pub struct BandaParametricaDef {
    pub frecuencia_hz: f64,
    pub factor_q: f64,
    pub ganancia_db: f64,
    pub tipo: TipoFiltro,
}

/// Definicion de un preset parametrico (PEQ).
#[derive(Debug, Clone, Copy)]
pub struct PresetParametricoDef {
    pub nombre: &'static str,
    pub bandas: &'static [BandaParametricaDef],
    pub preamp_db: f64,
    pub descripcion: &'static str,
}

/// Catalogo oficial de los 8 presets parametricos maestros de Poweramp.
pub const PRESETS_POWERAMP_PARAMETRICOS: [PresetParametricoDef; 8] = [
    PresetParametricoDef {
        nombre: "PEQ: Harman Reference",
        bandas: &[
            BandaParametricaDef { frecuencia_hz: 105.0, factor_q: 0.71, ganancia_db: 5.5, tipo: TipoFiltro::EstanteGraves },
            BandaParametricaDef { frecuencia_hz: 200.0, factor_q: 1.00, ganancia_db: -1.5, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 1200.0, factor_q: 1.20, ganancia_db: 1.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 3000.0, factor_q: 1.41, ganancia_db: 4.5, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 6000.0, factor_q: 2.00, ganancia_db: -2.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 10000.0, factor_q: 0.71, ganancia_db: 1.5, tipo: TipoFiltro::EstanteAgudos },
        ],
        preamp_db: -3.0,
        descripcion: "Curva objetivo Harman con estantes Butterworth y ganancia pinna",
    },
    PresetParametricoDef {
        nombre: "PEQ: Bass Warmth & Sub-Punch",
        bandas: &[
            BandaParametricaDef { frecuencia_hz: 35.0, factor_q: 0.80, ganancia_db: 6.5, tipo: TipoFiltro::EstanteGraves },
            BandaParametricaDef { frecuencia_hz: 75.0, factor_q: 1.30, ganancia_db: 4.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 220.0, factor_q: 1.10, ganancia_db: -2.5, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 2500.0, factor_q: 1.20, ganancia_db: 2.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 11000.0, factor_q: 0.71, ganancia_db: 3.0, tipo: TipoFiltro::EstanteAgudos },
        ],
        preamp_db: -4.0,
        descripcion: "Sub-graves extendidos con filtro anti-barro en 220 Hz",
    },
    PresetParametricoDef {
        nombre: "PEQ: Vocal Clarity & Air",
        bandas: &[
            BandaParametricaDef { frecuencia_hz: 80.0, factor_q: 0.71, ganancia_db: 0.0, tipo: TipoFiltro::PasoAlto },
            BandaParametricaDef { frecuencia_hz: 320.0, factor_q: 1.50, ganancia_db: -3.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 1500.0, factor_q: 1.20, ganancia_db: 2.5, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 3500.0, factor_q: 1.40, ganancia_db: 4.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 7500.0, factor_q: 2.50, ganancia_db: -2.5, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 14000.0, factor_q: 0.71, ganancia_db: 5.0, tipo: TipoFiltro::EstanteAgudos },
        ],
        preamp_db: -2.5,
        descripcion: "Filtro pasa-altos 80Hz, supresion de caja y aire en 14kHz",
    },
    PresetParametricoDef {
        nombre: "PEQ: Acoustic Intimate",
        bandas: &[
            BandaParametricaDef { frecuencia_hz: 50.0, factor_q: 0.71, ganancia_db: -2.0, tipo: TipoFiltro::EstanteGraves },
            BandaParametricaDef { frecuencia_hz: 140.0, factor_q: 1.10, ganancia_db: 3.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 650.0, factor_q: 1.20, ganancia_db: -2.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 2800.0, factor_q: 1.30, ganancia_db: 3.5, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 8500.0, factor_q: 0.80, ganancia_db: 2.5, tipo: TipoFiltro::EstanteAgudos },
        ],
        preamp_db: -2.0,
        descripcion: "Resonancia calida de caja acustica y ataque de cuerdas",
    },
    PresetParametricoDef {
        nombre: "PEQ: Electronic EDM Club",
        bandas: &[
            BandaParametricaDef { frecuencia_hz: 42.0, factor_q: 1.20, ganancia_db: 8.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 95.0, factor_q: 1.40, ganancia_db: 5.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 450.0, factor_q: 1.00, ganancia_db: -3.5, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 3200.0, factor_q: 1.30, ganancia_db: 3.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 12500.0, factor_q: 0.71, ganancia_db: 6.0, tipo: TipoFiltro::EstanteAgudos },
        ],
        preamp_db: -5.0,
        descripcion: "Sub-graves viscerales a 42Hz, pegada a 95Hz y brillo extremo",
    },
    PresetParametricoDef {
        nombre: "PEQ: Rock Punch & Bite",
        bandas: &[
            BandaParametricaDef { frecuencia_hz: 60.0, factor_q: 1.30, ganancia_db: 5.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 120.0, factor_q: 1.20, ganancia_db: 3.5, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 750.0, factor_q: 1.10, ganancia_db: -4.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 2600.0, factor_q: 1.40, ganancia_db: 4.5, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 9000.0, factor_q: 0.80, ganancia_db: 3.0, tipo: TipoFiltro::EstanteAgudos },
        ],
        preamp_db: -3.5,
        descripcion: "Bite de guitarra electrica en 2.6kHz y scoop de medios",
    },
    PresetParametricoDef {
        nombre: "PEQ: Gaming Tactical & Steps",
        bandas: &[
            BandaParametricaDef { frecuencia_hz: 65.0, factor_q: 1.00, ganancia_db: -3.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 120.0, factor_q: 2.00, ganancia_db: 4.5, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 1500.0, factor_q: 1.50, ganancia_db: 3.5, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 3800.0, factor_q: 2.00, ganancia_db: 6.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 8000.0, factor_q: 1.50, ganancia_db: 3.0, tipo: TipoFiltro::PicoParametrico },
        ],
        preamp_db: -3.0,
        descripcion: "Realce quirurgico de pisadas en 120Hz y disparos en 3.8kHz",
    },
    PresetParametricoDef {
        nombre: "PEQ: Monitor Neutral 10-Band",
        bandas: &[
            BandaParametricaDef { frecuencia_hz: 31.5, factor_q: 1.41, ganancia_db: 0.0, tipo: TipoFiltro::EstanteGraves },
            BandaParametricaDef { frecuencia_hz: 63.0, factor_q: 1.41, ganancia_db: 0.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 125.0, factor_q: 1.41, ganancia_db: 0.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 250.0, factor_q: 1.41, ganancia_db: 0.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 500.0, factor_q: 1.41, ganancia_db: 0.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 1000.0, factor_q: 1.41, ganancia_db: 0.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 2000.0, factor_q: 1.41, ganancia_db: 0.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 4000.0, factor_q: 1.41, ganancia_db: 0.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 8000.0, factor_q: 1.41, ganancia_db: 0.0, tipo: TipoFiltro::PicoParametrico },
            BandaParametricaDef { frecuencia_hz: 16000.0, factor_q: 1.41, ganancia_db: 0.0, tipo: TipoFiltro::EstanteAgudos },
        ],
        preamp_db: 0.0,
        descripcion: "Monitor neutro de estudio de 10 bandas parametricas a 0 dB",
    },
];

/// Localiza el archivo `auto_eq.db` en multiples rutas relativas y absolutas posibles.
fn localizar_archivo_autoeq(base: &std::path::Path) -> Option<PathBuf> {
    let mut candidatos = vec![
        base.join("recursos").join("auto_eq.db"),
        base.join("auto_eq.db"),
        PathBuf::from("recursos/auto_eq.db"),
        PathBuf::from("../recursos/auto_eq.db"),
        PathBuf::from("../../recursos/auto_eq.db"),
        PathBuf::from("/home/kali/EcualizadorApoEscritorio/recursos/auto_eq.db"),
        PathBuf::from("/home/kali/EcualizadorApoEscritorio/motor_rust/recursos/auto_eq.db"),
    ];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(p) = exe.parent() {
            candidatos.push(p.join("recursos").join("auto_eq.db"));
            candidatos.push(p.join("auto_eq.db"));
            if let Some(pp) = p.parent() {
                candidatos.push(pp.join("recursos").join("auto_eq.db"));
                if let Some(ppp) = pp.parent() {
                    candidatos.push(ppp.join("recursos").join("auto_eq.db"));
                }
            }
        }
    }
    candidatos.into_iter().find(|c| c.exists())
}

/// Mensajes de eventos e interacciones de la interfaz Iced.
#[derive(Debug, Clone)]
pub enum MensajeUi {
    CambiarEscena(EscenaActual),
    TickAnimacion(std::time::Instant),
    ConmutarEcualizador(bool),
    ConmutarLimitador(bool),
    ConmutarModoParametrico(bool),
    ResetearPlano,
    CambiarPreamp(f64),
    CambiarGananciaBanda(usize, f64),
    CambiarFrecuenciaBanda(usize, f64),
    CambiarFactorQBanda(usize, f64),
    CambiarTipoFiltroBanda(usize, TipoFiltro),
    SeleccionarBandaParaEdicion(usize),
    AgregarBandaParametrica,
    EliminarBandaParametrica(usize),
    AplicarPresetFabrica(&'static str),
    AplicarPresetParametrico(&'static str),
    ConmutarSubpestanaPresets(bool),
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
    AplicarPresetCompresor(&'static str),
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
    banda_seleccionada_idx: usize,
    subpestana_presets_parametrico: bool,
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
            consulta_autoeq: String::new(),
            resultados_autoeq: Vec::new(),
            mensaje_notificacion: String::from("Motor DSP listo a 64 bits"),
            barras_espectro_suavizadas: [0.0; 64],
            curva_magnitud_biquad: Vec::with_capacity(128),
            reduccion_compresor_db: 0.0,
            pico_salida_db: -96.0,
            estado_pipewire,
            pipewire_disponible,
            banda_seleccionada_idx: 0,
            subpestana_presets_parametrico: false,
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
        if let Some(ruta_db) = localizar_archivo_autoeq(&self.directorio_base) {
            if let Ok(res) = buscar_presets_autoeq_en_archivo(&ruta_db, &self.consulta_autoeq, 25) {
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
            MensajeUi::ConmutarModoParametrico(modo) => {
                self.estado_local.modo_parametrico = modo;
                if !modo {
                    // Al conmutar a Modo Grafico, asegurar las 10 bandas ISO estandar
                    let frecs_iso = [31.5, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0];
                    if self.estado_local.bandas.len() != 10 {
                        self.estado_local.bandas = frecs_iso
                            .iter()
                            .enumerate()
                            .map(|(i, &f)| {
                                BandaParametrica::nueva(
                                    i + 1,
                                    f,
                                    1.41,
                                    0.0,
                                    if i == 0 {
                                        TipoFiltro::EstanteGraves
                                    } else if i == 9 {
                                        TipoFiltro::EstanteAgudos
                                    } else {
                                        TipoFiltro::PicoParametrico
                                    },
                                )
                            })
                            .collect();
                    } else {
                        for (i, &f) in frecs_iso.iter().enumerate() {
                            self.estado_local.bandas[i].frecuencia_hz = f;
                            self.estado_local.bandas[i].factor_q = 1.41;
                            if i == 0 {
                                self.estado_local.bandas[i].tipo = TipoFiltro::EstanteGraves;
                            } else if i == 9 {
                                self.estado_local.bandas[i].tipo = TipoFiltro::EstanteAgudos;
                            } else {
                                self.estado_local.bandas[i].tipo = TipoFiltro::PicoParametrico;
                            }
                        }
                    }
                }
                self.sincronizar_dsp_e_ipc();
                self.mensaje_notificacion = format!(
                    "Modo ecualizador: {}",
                    if modo { "Parametrico Continuo (Fc, Q, Filtros)" } else { "Grafico Fijo (10 Bandas ISO)" }
                );
            }
            MensajeUi::ConmutarSubpestanaPresets(es_param) => {
                self.subpestana_presets_parametrico = es_param;
            }
            MensajeUi::ResetearPlano => {
                let frecs_iso = [31.5, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0];
                self.estado_local.bandas = frecs_iso
                    .iter()
                    .enumerate()
                    .map(|(i, &f)| {
                        BandaParametrica::nueva(
                            i + 1,
                            f,
                            1.41,
                            0.0,
                            if i == 0 {
                                TipoFiltro::EstanteGraves
                            } else if i == 9 {
                                TipoFiltro::EstanteAgudos
                            } else {
                                TipoFiltro::PicoParametrico
                            },
                        )
                    })
                    .collect();
                self.estado_local.ganancia_preamp_db = 0.0;
                self.estado_local.tono.graves_ganancia_db = 0.0;
                self.estado_local.tono.agudos_ganancia_db = 0.0;
                self.estado_local.nombre_perfil = String::from("Plano (0 dB)");
                self.estado_local.modo_parametrico = false;
                self.banda_seleccionada_idx = 0;
                self.escena_activa = EscenaActual::Ecualizador;
                self.sincronizar_dsp_e_ipc();
                self.mensaje_notificacion = String::from("Ecualizador reiniciado a Plano puro 0 dB");
            }
            MensajeUi::SeleccionarBandaParaEdicion(idx) => {
                if idx < self.estado_local.bandas.len() {
                    self.banda_seleccionada_idx = idx;
                }
            }
            MensajeUi::CambiarTipoFiltroBanda(idx, tipo) => {
                if idx < self.estado_local.bandas.len() {
                    self.estado_local.bandas[idx].tipo = tipo;
                    self.sincronizar_dsp_e_ipc();
                }
            }
            MensajeUi::AgregarBandaParametrica => {
                if self.estado_local.bandas.len() < 32 {
                    let id = self.estado_local.bandas.len() + 1;
                    let nueva = BandaParametrica::nueva(
                        id,
                        1000.0,
                        1.41,
                        0.0,
                        TipoFiltro::PicoParametrico,
                    );
                    self.estado_local.bandas.push(nueva);
                    self.banda_seleccionada_idx = self.estado_local.bandas.len() - 1;
                    self.estado_local.modo_parametrico = true;
                    self.sincronizar_dsp_e_ipc();
                    self.mensaje_notificacion = format!("Banda {} agregada", id);
                }
            }
            MensajeUi::EliminarBandaParametrica(idx) => {
                if self.estado_local.bandas.len() > 1 && idx < self.estado_local.bandas.len() {
                    self.estado_local.bandas.remove(idx);
                    for (i, b) in self.estado_local.bandas.iter_mut().enumerate() {
                        b.identificador = i + 1;
                    }
                    if self.banda_seleccionada_idx >= self.estado_local.bandas.len() {
                        self.banda_seleccionada_idx = self.estado_local.bandas.len().saturating_sub(1);
                    }
                    self.sincronizar_dsp_e_ipc();
                    self.mensaje_notificacion = String::from("Banda eliminada");
                }
            }
            MensajeUi::AplicarPresetFabrica(nombre) => {
                if let Some(p) = PRESETS_POWERAMP_GRAFICOS.iter().find(|p| p.nombre == nombre) {
                    let frecs_iso = [31.5, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0];
                    self.estado_local.bandas = frecs_iso
                        .iter()
                        .enumerate()
                        .map(|(i, &f)| {
                            BandaParametrica::nueva(
                                i + 1,
                                f,
                                1.41,
                                p.ganancias_db[i],
                                if i == 0 {
                                    TipoFiltro::EstanteGraves
                                } else if i == 9 {
                                    TipoFiltro::EstanteAgudos
                                } else {
                                    TipoFiltro::PicoParametrico
                                },
                            )
                        })
                        .collect();
                    self.estado_local.ganancia_preamp_db = p.preamp_db;
                    self.estado_local.nombre_perfil = format!("Preset: {}", p.nombre);
                    self.estado_local.modo_parametrico = false;
                    self.estado_local.ecualizador_activo = true;
                    self.banda_seleccionada_idx = 0;
                    self.escena_activa = EscenaActual::Ecualizador;
                    self.sincronizar_dsp_e_ipc();
                    self.mensaje_notificacion = format!("Preset '{}' aplicado (10 bandas ISO)", p.nombre);
                }
            }
            MensajeUi::AplicarPresetParametrico(nombre) => {
                if let Some(p) = PRESETS_POWERAMP_PARAMETRICOS.iter().find(|p| p.nombre == nombre) {
                    self.estado_local.bandas = p
                        .bandas
                        .iter()
                        .enumerate()
                        .map(|(i, b)| {
                            BandaParametrica::nueva(
                                i + 1,
                                b.frecuencia_hz,
                                b.factor_q,
                                b.ganancia_db,
                                b.tipo,
                            )
                        })
                        .collect();
                    self.estado_local.ganancia_preamp_db = p.preamp_db;
                    self.estado_local.nombre_perfil = format!("PEQ: {}", p.nombre);
                    self.estado_local.modo_parametrico = true;
                    self.estado_local.ecualizador_activo = true;
                    self.banda_seleccionada_idx = 0;
                    self.escena_activa = EscenaActual::Ecualizador;
                    self.sincronizar_dsp_e_ipc();
                    self.mensaje_notificacion = format!(
                        "Preset Parametrico '{}' aplicado ({} bandas)",
                        p.nombre,
                        p.bandas.len()
                    );
                }
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
                self.estado_local.tono.tono_activo = true;
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::CambiarTonoAgudos(ganancia_db) => {
                self.estado_local.tono.agudos_ganancia_db = ganancia_db;
                self.estado_local.tono.tono_activo = true;
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
                if db < -0.1 {
                    self.estado_local.compresor.activo = true;
                }
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
                if db.abs() > 0.1 {
                    self.estado_local.compresor.activo = true;
                }
                self.sincronizar_dsp_e_ipc();
            }
            MensajeUi::AplicarPresetCompresor(nombre) => {
                match nombre {
                    "Suave / Mastering" => {
                        self.estado_local.compresor.activo = true;
                        self.estado_local.compresor.limite_db = -12.0;
                        self.estado_local.compresor.proporcion = 2.0;
                        self.estado_local.compresor.ataque_ms = 30.0;
                        self.estado_local.compresor.lanzamiento_ms = 150.0;
                        self.estado_local.compresor.frecuencia_sidechain_hz = 90.0;
                        self.estado_local.compresor.ganancia_maquillaje_db = 2.0;
                    }
                    "Pegada / Punch" => {
                        self.estado_local.compresor.activo = true;
                        self.estado_local.compresor.limite_db = -20.0;
                        self.estado_local.compresor.proporcion = 4.0;
                        self.estado_local.compresor.ataque_ms = 10.0;
                        self.estado_local.compresor.lanzamiento_ms = 80.0;
                        self.estado_local.compresor.frecuencia_sidechain_hz = 120.0;
                        self.estado_local.compresor.ganancia_maquillaje_db = 4.5;
                    }
                    "Radio / Agresivo" => {
                        self.estado_local.compresor.activo = true;
                        self.estado_local.compresor.limite_db = -28.0;
                        self.estado_local.compresor.proporcion = 8.0;
                        self.estado_local.compresor.ataque_ms = 2.0;
                        self.estado_local.compresor.lanzamiento_ms = 40.0;
                        self.estado_local.compresor.frecuencia_sidechain_hz = 160.0;
                        self.estado_local.compresor.ganancia_maquillaje_db = 8.0;
                    }
                    _ => {}
                }
                self.sincronizar_dsp_e_ipc();
                self.mensaje_notificacion = format!("Compresor: preset '{}' activado", nombre);
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
                if mezcla > 0.005 {
                    self.estado_local.reverberacion.activo = true;
                }
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
                    "Placa" | "Placa Metalica" => {
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
                self.mensaje_notificacion = format!("Reverberacion: preset '{}' activado", nombre);
            }
            MensajeUi::ConsultaAutoEqCambiada(texto) => {
                self.consulta_autoeq = texto;
            }
            MensajeUi::EjecutarBusquedaAutoEq => {
                if let Some(ruta_db) = localizar_archivo_autoeq(&self.directorio_base) {
                    match buscar_presets_autoeq_en_archivo(&ruta_db, &self.consulta_autoeq, 40) {
                        Ok(res) => {
                            let total = res.len();
                            self.resultados_autoeq = res;
                            self.mensaje_notificacion = format!(
                                "Se encontraron {} perfiles en auto_eq.db para '{}'",
                                total, self.consulta_autoeq
                            );
                        }
                        Err(e) => {
                            self.mensaje_notificacion = format!("Error al buscar: {}", e);
                        }
                    }
                } else {
                    self.mensaje_notificacion = String::from("No se encontro auto_eq.db en el disco");
                }
            }
            MensajeUi::CargarPerfilAutoEq(nombre_modelo) => {
                if let Some(ruta_db) = localizar_archivo_autoeq(&self.directorio_base) {
                    match cargar_preset_autoeq_por_consulta(&ruta_db, &nombre_modelo) {
                        Ok(mut nuevo_estado) => {
                            nuevo_estado.ecualizador_activo = true;
                            let nombre = nuevo_estado.nombre_perfil.clone();
                            let es_param = nuevo_estado.modo_parametrico;
                            let num_bandas = nuevo_estado.bandas.len();
                            self.estado_local = nuevo_estado;
                            self.banda_seleccionada_idx = 0;
                            self.escena_activa = EscenaActual::Ecualizador;
                            self.sincronizar_dsp_e_ipc();
                            self.mensaje_notificacion = format!(
                                "Perfil AutoEQ '{}' aplicado con exito ({} bandas {})",
                                nombre,
                                num_bandas,
                                if es_param { "Parametrico" } else { "Grafico" }
                            );
                        }
                        Err(e) => {
                            self.mensaje_notificacion = format!("Error al cargar preset: {}", e);
                        }
                    }
                } else {
                    self.mensaje_notificacion = String::from("No se encontro auto_eq.db en el disco");
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
        let titulo = text("ALDRA POWERAMP")
            .size(17)
            .color(COLOR_AMBAR_SUNLIGHT);

        // Boton Maestro EQU (On/Off)
        let eq_on = self.estado_local.ecualizador_activo;
        let btn_equ = button(
            text(if eq_on { "EQU [ON]" } else { "EQU [BYP]" })
                .size(11)
                .color(if eq_on { Color::BLACK } else { COLOR_TEXTO_ATENUADO }),
        )
        .padding([4, 10])
        .style(move |_t: &Theme, _s: button::Status| {
            if eq_on {
                button::Style {
                    background: Some(Background::Color(COLOR_AMBAR_SUNLIGHT)),
                    border: Border { radius: 4.0.into(), ..Default::default() },
                    ..Default::default()
                }
            } else {
                button::Style {
                    background: Some(Background::Color(Color::from_rgb(0.08, 0.10, 0.14))),
                    border: Border { color: Color::from_rgba(0.3, 0.35, 0.45, 0.3), width: 1.0, radius: 4.0.into() },
                    ..Default::default()
                }
            }
        })
        .on_press(MensajeUi::ConmutarEcualizador(!eq_on));

        // Boton Maestro TONE (On/Off)
        let tone_on = self.estado_local.tono.tono_activo;
        let btn_tone = button(
            text(if tone_on { "TONE [ON]" } else { "TONE [BYP]" })
                .size(11)
                .color(if tone_on { Color::BLACK } else { COLOR_TEXTO_ATENUADO }),
        )
        .padding([4, 10])
        .style(move |_t: &Theme, _s: button::Status| {
            if tone_on {
                button::Style {
                    background: Some(Background::Color(COLOR_CIAN_ELECTRICO)),
                    border: Border { radius: 4.0.into(), ..Default::default() },
                    ..Default::default()
                }
            } else {
                button::Style {
                    background: Some(Background::Color(Color::from_rgb(0.08, 0.10, 0.14))),
                    border: Border { color: Color::from_rgba(0.3, 0.35, 0.45, 0.3), width: 1.0, radius: 4.0.into() },
                    ..Default::default()
                }
            }
        })
        .on_press(MensajeUi::ConmutarTono(!tone_on));

        // Boton Maestro LIMIT (On/Off)
        let lim_on = self.estado_local.limitador_activo;
        let btn_limit = button(
            text(if lim_on { "LIMIT [ON]" } else { "LIMIT [BYP]" })
                .size(11)
                .color(if lim_on { Color::BLACK } else { COLOR_TEXTO_ATENUADO }),
        )
        .padding([4, 10])
        .style(move |_t: &Theme, _s: button::Status| {
            if lim_on {
                button::Style {
                    background: Some(Background::Color(COLOR_CHERRY_NEON)),
                    border: Border { radius: 4.0.into(), ..Default::default() },
                    ..Default::default()
                }
            } else {
                button::Style {
                    background: Some(Background::Color(Color::from_rgb(0.08, 0.10, 0.14))),
                    border: Border { color: Color::from_rgba(0.3, 0.35, 0.45, 0.3), width: 1.0, radius: 4.0.into() },
                    ..Default::default()
                }
            }
        })
        .on_press(MensajeUi::ConmutarLimitador(!lim_on));

        // Boton Modo: Grafico vs Parametrico
        let es_param = self.estado_local.modo_parametrico;
        let btn_modo = button(
            text(if es_param { "MODO: PARAMETRICO" } else { "MODO: GRAFICO" })
                .size(11)
                .color(COLOR_TEXTO_CLARO),
        )
        .padding([4, 10])
        .style(move |_t: &Theme, _s: button::Status| {
            button::Style {
                background: Some(Background::Color(Color::from_rgb(0.12, 0.15, 0.22))),
                border: Border {
                    color: if es_param { COLOR_CIAN_ELECTRICO } else { COLOR_AMBAR_SUNLIGHT },
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            }
        })
        .on_press(MensajeUi::ConmutarModoParametrico(!es_param));

        // Boton Preset Activo (Abre Pestaña de Presets & AutoEQ)
        let btn_preset = button(
            text(format!("Preset: {}", self.estado_local.nombre_perfil))
                .size(11)
                .color(COLOR_AMBAR_SUNLIGHT),
        )
        .padding([4, 10])
        .style(|_t: &Theme, _s: button::Status| button::Style {
            background: Some(Background::Color(Color::from_rgb(0.09, 0.11, 0.16))),
            border: Border {
                color: Color::from_rgba(0.96, 0.62, 0.04, 0.5),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .on_press(MensajeUi::CambiarEscena(EscenaActual::AutoEqDispositivos));

        // Boton Reset Plano (0 dB)
        let btn_reset = button(text("0 dB PLANO").size(11).color(COLOR_TEXTO_CLARO))
            .padding([4, 8])
            .style(|_t: &Theme, _s: button::Status| button::Style {
                background: Some(Background::Color(Color::from_rgb(0.15, 0.18, 0.24))),
                border: Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            })
            .on_press(MensajeUi::ResetearPlano);

        // Boton Exportar config.txt
        let btn_exportar = button(text("APO config.txt").size(11).color(Color::BLACK))
            .padding([4, 8])
            .style(|_t: &Theme, _s: button::Status| button::Style {
                background: Some(Background::Color(COLOR_CIAN_ELECTRICO)),
                border: Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            })
            .on_press(MensajeUi::GuardarPresetApo);

        row![
            titulo,
            space::horizontal(),
            btn_equ,
            btn_tone,
            btn_limit,
            btn_modo,
            btn_preset,
            btn_reset,
            btn_exportar,
        ]
        .spacing(8)
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
            text("4. Presets & AutoEQ")
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

        // 2. Barra de Presets Rapidos de Fabrica (Poweramp 23 Graficos + 8 Parametricos)
        let es_param_presets = self.subpestana_presets_parametrico;

        let btn_sel_graf = button(
            text("GRAFICOS (23)")
                .size(10)
                .color(if !es_param_presets { Color::BLACK } else { COLOR_TEXTO_ATENUADO }),
        )
        .padding([3, 8])
        .style(move |_t: &Theme, _s: button::Status| {
            if !es_param_presets {
                button::Style {
                    background: Some(Background::Color(COLOR_AMBAR_SUNLIGHT)),
                    border: Border { radius: 3.0.into(), ..Default::default() },
                    ..Default::default()
                }
            } else {
                button::Style {
                    background: Some(Background::Color(Color::from_rgb(0.08, 0.10, 0.14))),
                    border: Border { radius: 3.0.into(), ..Default::default() },
                    ..Default::default()
                }
            }
        })
        .on_press(MensajeUi::ConmutarSubpestanaPresets(false));

        let btn_sel_param = button(
            text("PARAMETRICOS (8)")
                .size(10)
                .color(if es_param_presets { Color::BLACK } else { COLOR_TEXTO_ATENUADO }),
        )
        .padding([3, 8])
        .style(move |_t: &Theme, _s: button::Status| {
            if es_param_presets {
                button::Style {
                    background: Some(Background::Color(COLOR_CIAN_ELECTRICO)),
                    border: Border { radius: 3.0.into(), ..Default::default() },
                    ..Default::default()
                }
            } else {
                button::Style {
                    background: Some(Background::Color(Color::from_rgb(0.08, 0.10, 0.14))),
                    border: Border { radius: 3.0.into(), ..Default::default() },
                    ..Default::default()
                }
            }
        })
        .on_press(MensajeUi::ConmutarSubpestanaPresets(true));

        let btn_ver_todos = button(
            text("+ BIBLIOTECA COMPLETA")
                .size(10)
                .color(COLOR_TEXTO_CLARO),
        )
        .padding([3, 8])
        .style(|_t: &Theme, _s: button::Status| button::Style {
            background: Some(Background::Color(Color::from_rgb(0.12, 0.15, 0.22))),
            border: Border {
                color: Color::from_rgba(0.96, 0.62, 0.04, 0.4),
                width: 1.0,
                radius: 3.0.into(),
            },
            ..Default::default()
        })
        .on_press(MensajeUi::CambiarEscena(EscenaActual::AutoEqDispositivos));

        let mut fila_presets = row![btn_sel_graf, btn_sel_param, btn_ver_todos].spacing(6);

        if !es_param_presets {
            for p in &PRESETS_POWERAMP_GRAFICOS {
                let p_nom = p.nombre;
                let es_activo = self.estado_local.nombre_perfil.contains(p_nom);
                let btn = button(
                    text(p_nom)
                        .size(11)
                        .color(if es_activo { Color::BLACK } else { COLOR_TEXTO_CLARO }),
                )
                .padding([4, 10])
                .style(move |_t: &Theme, _s: button::Status| {
                    if es_activo {
                        button::Style {
                            background: Some(Background::Color(COLOR_AMBAR_SUNLIGHT)),
                            border: Border {
                                radius: 4.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }
                    } else {
                        button::Style {
                            background: Some(Background::Color(Color::from_rgb(0.10, 0.13, 0.18))),
                            border: Border {
                                color: Color::from_rgba(0.3, 0.35, 0.45, 0.3),
                                width: 1.0,
                                radius: 4.0.into(),
                            },
                            ..Default::default()
                        }
                    }
                })
                .on_press(MensajeUi::AplicarPresetFabrica(p_nom));
                fila_presets = fila_presets.push(btn);
            }
        } else {
            for p in &PRESETS_POWERAMP_PARAMETRICOS {
                let p_nom = p.nombre;
                let es_activo = self.estado_local.nombre_perfil.contains(p_nom);
                let btn = button(
                    text(p_nom)
                        .size(11)
                        .color(if es_activo { Color::BLACK } else { COLOR_TEXTO_CLARO }),
                )
                .padding([4, 10])
                .style(move |_t: &Theme, _s: button::Status| {
                    if es_activo {
                        button::Style {
                            background: Some(Background::Color(COLOR_CIAN_ELECTRICO)),
                            border: Border {
                                radius: 4.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }
                    } else {
                        button::Style {
                            background: Some(Background::Color(Color::from_rgb(0.10, 0.13, 0.18))),
                            border: Border {
                                color: Color::from_rgba(0.02, 0.71, 0.83, 0.3),
                                width: 1.0,
                                radius: 4.0.into(),
                            },
                            ..Default::default()
                        }
                    }
                })
                .on_press(MensajeUi::AplicarPresetParametrico(p_nom));
                fila_presets = fila_presets.push(btn);
            }
        }

        let barra_presets = scrollable(fila_presets).direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::default(),
        ));

        // 3. Barra de Control de Modo (Grafico vs Parametrico) y Boton de Agregar Banda
        let es_param = self.estado_local.modo_parametrico;
        let btn_modo_grafico = button(
            text("MODO GRAFICO (10 BANDAS ISO)")
                .size(11)
                .color(if !es_param { Color::BLACK } else { COLOR_TEXTO_ATENUADO }),
        )
        .padding([4, 12])
        .style(move |_t: &Theme, _s: button::Status| {
            if !es_param {
                button::Style {
                    background: Some(Background::Color(COLOR_AMBAR_SUNLIGHT)),
                    border: Border { radius: 4.0.into(), ..Default::default() },
                    ..Default::default()
                }
            } else {
                button::Style {
                    background: Some(Background::Color(Color::from_rgb(0.08, 0.10, 0.14))),
                    border: Border { color: Color::from_rgba(0.25, 0.3, 0.4, 0.3), width: 1.0, radius: 4.0.into() },
                    ..Default::default()
                }
            }
        })
        .on_press(MensajeUi::ConmutarModoParametrico(false));

        let btn_modo_param = button(
            text("MODO PARAMETRICO (Hz, Q, dB, Filtros)")
                .size(11)
                .color(if es_param { Color::BLACK } else { COLOR_TEXTO_ATENUADO }),
        )
        .padding([4, 12])
        .style(move |_t: &Theme, _s: button::Status| {
            if es_param {
                button::Style {
                    background: Some(Background::Color(COLOR_AMBAR_SUNLIGHT)),
                    border: Border { radius: 4.0.into(), ..Default::default() },
                    ..Default::default()
                }
            } else {
                button::Style {
                    background: Some(Background::Color(Color::from_rgb(0.08, 0.10, 0.14))),
                    border: Border { color: Color::from_rgba(0.25, 0.3, 0.4, 0.3), width: 1.0, radius: 4.0.into() },
                    ..Default::default()
                }
            }
        })
        .on_press(MensajeUi::ConmutarModoParametrico(true));

        let btn_agregar_banda = button(
            text("+ AGREGAR BANDA")
                .size(11)
                .color(Color::BLACK),
        )
        .padding([4, 12])
        .style(|_t: &Theme, _s: button::Status| button::Style {
            background: Some(Background::Color(COLOR_CIAN_ELECTRICO)),
            border: Border { radius: 4.0.into(), ..Default::default() },
            ..Default::default()
        })
        .on_press(MensajeUi::AgregarBandaParametrica);

        let barra_modos = row![
            btn_modo_grafico,
            btn_modo_param,
            space::horizontal(),
            if es_param {
                Element::from(btn_agregar_banda)
            } else {
                Element::from(text(format!("Perfil: {}", self.estado_local.nombre_perfil)).size(12).color(COLOR_TEXTO_ATENUADO))
            }
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        // 4. Panel Inspector de Banda Parametrica Seleccionada (solo visible en Modo Parametrico)
        let inspector_banda: Option<Element<'_, MensajeUi>> = if es_param {
            let idx_sel = self.banda_seleccionada_idx.min(self.estado_local.bandas.len().saturating_sub(1));
            if let Some(banda_sel) = self.estado_local.bandas.get(idx_sel) {
                let tipo_actual = banda_sel.tipo;
                let tipos_disponibles = [
                    (TipoFiltro::PicoParametrico, "Pico (Bell)"),
                    (TipoFiltro::EstanteGraves, "Low Shelf"),
                    (TipoFiltro::EstanteAgudos, "High Shelf"),
                    (TipoFiltro::PasoBajo, "Low Pass"),
                    (TipoFiltro::PasoAlto, "High Pass"),
                    (TipoFiltro::PasoBanda, "Band Pass"),
                ];

                let mut fila_tipos = row![].spacing(4);
                for (tipo, etiqueta) in tipos_disponibles {
                    let es_tipo_activo = tipo == tipo_actual;
                    let btn_tipo = button(
                        text(etiqueta)
                            .size(10)
                            .color(if es_tipo_activo { Color::BLACK } else { COLOR_TEXTO_CLARO }),
                    )
                    .padding([3, 8])
                    .style(move |_t: &Theme, _s: button::Status| {
                        if es_tipo_activo {
                            button::Style {
                                background: Some(Background::Color(COLOR_AMBAR_SUNLIGHT)),
                                border: Border { radius: 3.0.into(), ..Default::default() },
                                ..Default::default()
                            }
                        } else {
                            button::Style {
                                background: Some(Background::Color(Color::from_rgb(0.12, 0.15, 0.20))),
                                border: Border { color: Color::from_rgba(0.3, 0.35, 0.45, 0.3), width: 1.0, radius: 3.0.into() },
                                ..Default::default()
                            }
                        }
                    })
                    .on_press(MensajeUi::CambiarTipoFiltroBanda(idx_sel, tipo));
                    fila_tipos = fila_tipos.push(btn_tipo);
                }

                let btn_eliminar = button(
                    text("Eliminar Banda X").size(10).color(Color::WHITE),
                )
                .padding([3, 8])
                .style(|_t: &Theme, _s: button::Status| button::Style {
                    background: Some(Background::Color(COLOR_CHERRY_NEON)),
                    border: Border { radius: 3.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .on_press(MensajeUi::EliminarBandaParametrica(idx_sel));

                let panel_editor = container(
                    column![
                        row![
                            text(format!(
                                "INSPECTOR PARAMETRICO: BANDA #{} ({:.0} Hz, {:+.1} dB, Q {:.2})",
                                banda_sel.identificador,
                                banda_sel.frecuencia_hz,
                                banda_sel.ganancia_db,
                                banda_sel.factor_q
                            ))
                            .size(12)
                            .color(COLOR_AMBAR_SUNLIGHT),
                            space::horizontal(),
                            if self.estado_local.bandas.len() > 1 {
                                Element::from(btn_eliminar)
                            } else {
                                Element::from(space::horizontal())
                            }
                        ]
                        .align_y(iced::Alignment::Center),
                        row![
                            text("Tipo:").size(11).color(COLOR_TEXTO_ATENUADO),
                            fila_tipos,
                        ]
                        .spacing(8)
                        .align_y(iced::Alignment::Center),
                        row![
                            // Control de Frecuencia (20 a 20000 Hz)
                            column![
                                text(format!("Frecuencia: {:.0} Hz", banda_sel.frecuencia_hz))
                                    .size(11)
                                    .color(COLOR_CIAN_ELECTRICO),
                                slider(
                                    20.0..=20000.0,
                                    banda_sel.frecuencia_hz,
                                    move |hz| MensajeUi::CambiarFrecuenciaBanda(idx_sel, hz),
                                )
                                .step(1.0)
                                .width(Length::Fixed(190.0)),
                            ]
                            .spacing(2),
                            // Control de Factor Q (0.1 a 10.0)
                            column![
                                text(format!("Factor Q: {:.2}", banda_sel.factor_q))
                                    .size(11)
                                    .color(COLOR_CIAN_ELECTRICO),
                                slider(
                                    0.1..=10.0,
                                    banda_sel.factor_q,
                                    move |q| MensajeUi::CambiarFactorQBanda(idx_sel, q),
                                )
                                .step(0.05)
                                .width(Length::Fixed(150.0)),
                            ]
                            .spacing(2),
                            // Control de Ganancia (-24 a +24 dB)
                            column![
                                text(format!("Ganancia: {:+.1} dB", banda_sel.ganancia_db))
                                    .size(11)
                                    .color(COLOR_AMBAR_SUNLIGHT),
                                slider(
                                    -24.0..=24.0,
                                    banda_sel.ganancia_db,
                                    move |db| MensajeUi::CambiarGananciaBanda(idx_sel, db),
                                )
                                .step(0.1)
                                .width(Length::Fixed(190.0)),
                            ]
                            .spacing(2),
                        ]
                        .spacing(16)
                        .align_y(iced::Alignment::Center),
                    ]
                    .spacing(8),
                )
                .padding(10)
                .style(|_t: &Theme| container::Style {
                    background: Some(Background::Color(Color::from_rgb(0.05, 0.07, 0.11))),
                    border: Border {
                        color: Color::from_rgba(0.96, 0.62, 0.04, 0.4),
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                });

                Some(panel_editor.into())
            } else {
                None
            }
        } else {
            None
        };

        // 5. Tira de Faders de Bandas (Obsidian Rack)
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
                .height(130.0),
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

        let idx_seleccionada = self.banda_seleccionada_idx;
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

            let es_la_seleccionada = es_param && idx == idx_seleccionada;
            let esta_activa = banda.activo;

            let btn_sel = if es_param {
                button(
                    text(format!("#{}", banda.identificador))
                        .size(9)
                        .color(if es_la_seleccionada { Color::BLACK } else { COLOR_TEXTO_CLARO }),
                )
                .padding([2, 6])
                .style(move |_t: &Theme, _s: button::Status| {
                    if es_la_seleccionada {
                        button::Style {
                            background: Some(Background::Color(COLOR_AMBAR_SUNLIGHT)),
                            border: Border { radius: 3.0.into(), ..Default::default() },
                            ..Default::default()
                        }
                    } else {
                        button::Style {
                            background: Some(Background::Color(Color::from_rgb(0.12, 0.15, 0.20))),
                            border: Border { radius: 3.0.into(), ..Default::default() },
                            ..Default::default()
                        }
                    }
                })
                .on_press(MensajeUi::SeleccionarBandaParaEdicion(idx))
            } else {
                button(text(format!("#{}", banda.identificador)).size(9).color(COLOR_TEXTO_ATENUADO))
            };

            let strip_banda = container(
                column![
                    btn_sel,
                    text(texto_db).size(11).color(color_ganancia),
                    text("+12").size(8).color(Color::from_rgba(0.5, 0.55, 0.65, 0.3)),
                    slider(
                        -24.0..=24.0,
                        banda.ganancia_db,
                        move |val| MensajeUi::CambiarGananciaBanda(idx, val),
                    )
                    .step(0.1)
                    .height(130.0),
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
                background: Some(Background::Color(if es_la_seleccionada {
                    Color::from_rgb(0.09, 0.12, 0.18)
                } else {
                    Color::from_rgb(0.07, 0.09, 0.13)
                })),
                border: Border {
                    color: if es_la_seleccionada {
                        COLOR_AMBAR_SUNLIGHT
                    } else if esta_activa {
                        Color::from_rgba(0.25, 0.30, 0.40, 0.35)
                    } else {
                        Color::from_rgba(0.15, 0.18, 0.25, 0.2)
                    },
                    width: if es_la_seleccionada { 1.5 } else { 1.0 },
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

        // 6. Controles Maestros de Tono (Graves y Agudos estilo Poweramp)
        let btn_reset_bass = button(text("0 dB").size(9).color(COLOR_TEXTO_CLARO))
            .padding([2, 6])
            .on_press(MensajeUi::CambiarTonoGraves(0.0));

        let btn_reset_treble = button(text("0 dB").size(9).color(COLOR_TEXTO_CLARO))
            .padding([2, 6])
            .on_press(MensajeUi::CambiarTonoAgudos(0.0));

        let tono_activo = self.estado_local.tono.tono_activo;
        let etiqueta_estado_tono = if tono_activo {
            text("TONO: ACTIVO").size(10).color(COLOR_CIAN_ELECTRICO)
        } else {
            text("TONO: BYPASS").size(10).color(COLOR_TEXTO_ATENUADO)
        };

        let controles_tono = row![
            column![
                row![text("BASS / GRAVES").size(12).color(COLOR_CIAN_ELECTRICO), btn_reset_bass]
                    .spacing(6)
                    .align_y(iced::Alignment::Center),
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
            etiqueta_estado_tono,
            space::horizontal(),
            column![
                row![text("TREBLE / AGUDOS").size(12).color(COLOR_CIAN_ELECTRICO), btn_reset_treble]
                    .spacing(6)
                    .align_y(iced::Alignment::Center),
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

        let mut contenido_columna = column![visualizador, barra_presets, barra_modos].spacing(10);
        if let Some(inspector) = inspector_banda {
            contenido_columna = contenido_columna.push(inspector);
        }
        contenido_columna = contenido_columna.push(panel_faders).push(controles_tono);

        contenido_columna.into()
    }

    fn construir_escena_espacio_compresor(&self) -> Element<'_, MensajeUi> {
        // Panel de Espacio Estereo
        let panel_espacio = column![
            row![
                text("PROCESADOR ESPACIAL MID / SIDE")
                    .size(16)
                    .color(COLOR_AMBAR_SUNLIGHT),
                space::horizontal(),
                button(text("Centrar (0.0)").size(11))
                    .on_press(MensajeUi::CambiarBalance(0.0)),
                button(text("100% Normal").size(11))
                    .on_press(MensajeUi::CambiarExpansionEstereo(1.0)),
                button(text("130% Amplio").size(11))
                    .on_press(MensajeUi::CambiarExpansionEstereo(1.3)),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(8),
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
        let presets_comp = row![
            text("PRESETS RAPIDOS:").size(11).color(COLOR_TEXTO_ATENUADO),
            button(text("Suave / Mastering").size(11))
                .on_press(MensajeUi::AplicarPresetCompresor("Suave / Mastering")),
            button(text("Pegada / Punch").size(11))
                .on_press(MensajeUi::AplicarPresetCompresor("Pegada / Punch")),
            button(text("Radio / Agresivo").size(11))
                .on_press(MensajeUi::AplicarPresetCompresor("Radio / Agresivo")),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

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
            row![
                text(format!(
                    "Reduccion de Ganancia Activa: -{:.1} dB",
                    self.reduccion_compresor_db
                ))
                .size(13)
                .color(COLOR_CHERRY_NEON),
                space::horizontal(),
                presets_comp,
            ]
            .align_y(iced::Alignment::Center),
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
        // 1. Selector de Subpestana de Presets Oficiales de Poweramp
        let es_param = self.subpestana_presets_parametrico;
        let btn_pestana_graf = button(
            text(format!("Presets Graficos ({})", PRESETS_POWERAMP_GRAFICOS.len()))
                .size(12)
                .color(if !es_param { COLOR_AMBAR_SUNLIGHT } else { COLOR_TEXTO_ATENUADO })
        )
        .on_press(MensajeUi::ConmutarSubpestanaPresets(false));

        let btn_pestana_param = button(
            text(format!("Presets Parametricos PEQ ({})", PRESETS_POWERAMP_PARAMETRICOS.len()))
                .size(12)
                .color(if es_param { COLOR_AMBAR_SUNLIGHT } else { COLOR_TEXTO_ATENUADO })
        )
        .on_press(MensajeUi::ConmutarSubpestanaPresets(true));

        let btn_reset_plano = button(text("Reset 0 dB Plano").size(12))
            .on_press(MensajeUi::ResetearPlano);

        let selector_presets = row![
            text("PRESETS OFICIALES POWERAMP:").size(13).color(COLOR_AMBAR_SUNLIGHT),
            btn_pestana_graf,
            btn_pestana_param,
            space::horizontal(),
            btn_reset_plano,
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center);

        // Cuadricula / Lista de Tarjetas de Presets
        let mut tarjetas_presets = column![].spacing(6);
        if !es_param {
            // 23 Presets Graficos Oficiales
            for preset in PRESETS_POWERAMP_GRAFICOS.iter() {
                let nombre = preset.nombre;
                let preamp = preset.preamp_db;
                let desc = preset.descripcion;
                let es_actual = self.estado_local.nombre_perfil == nombre && !self.estado_local.modo_parametrico;

                let fila = row![
                    column![
                        text(nombre).size(13).color(if es_actual { COLOR_AMBAR_SUNLIGHT } else { COLOR_TEXTO_CLARO }),
                        text(format!("Preamp: {:+.1} dB | {}", preamp, desc)).size(11).color(COLOR_TEXTO_ATENUADO),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                    button(text(if es_actual { "Activo" } else { "Aplicar" }).size(11))
                        .on_press(MensajeUi::AplicarPresetFabrica(nombre)),
                ]
                .align_y(iced::Alignment::Center)
                .padding(4);

                tarjetas_presets = tarjetas_presets.push(fila);
            }
        } else {
            // 8 Presets Parametricos Oficiales
            for preset in PRESETS_POWERAMP_PARAMETRICOS.iter() {
                let nombre = preset.nombre;
                let preamp = preset.preamp_db;
                let desc = preset.descripcion;
                let num_bandas = preset.bandas.len();
                let es_actual = self.estado_local.nombre_perfil == nombre && self.estado_local.modo_parametrico;

                let fila = row![
                    column![
                        text(nombre).size(13).color(if es_actual { COLOR_AMBAR_SUNLIGHT } else { COLOR_TEXTO_CLARO }),
                        text(format!("{} bandas PEQ | Preamp: {:+.1} dB | {}", num_bandas, preamp, desc)).size(11).color(COLOR_TEXTO_ATENUADO),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                    button(text(if es_actual { "Activo" } else { "Aplicar PEQ" }).size(11))
                        .on_press(MensajeUi::AplicarPresetParametrico(nombre)),
                ]
                .align_y(iced::Alignment::Center)
                .padding(4);

                tarjetas_presets = tarjetas_presets.push(fila);
            }
        }

        let panel_presets = column![
            selector_presets,
            scrollable(tarjetas_presets).height(Length::Fixed(180.0)),
        ]
        .spacing(8);

        // 2. Seccion AutoEQ (+20,531 modelos)
        let entrada_busqueda = text_input(
            "Buscar modelo de auriculares (ej. Sony WH-1000XM4, HD650, JBL, AirPods, Moondrop, KZ)...",
            &self.consulta_autoeq,
        )
        .on_input(MensajeUi::ConsultaAutoEqCambiada)
        .on_submit(MensajeUi::EjecutarBusquedaAutoEq)
        .padding(8);

        let btn_buscar = button(text("Buscar en DB").size(13))
            .on_press(MensajeUi::EjecutarBusquedaAutoEq);

        let mut lista_auriculares = column![].spacing(4);
        for item in &self.resultados_autoeq {
            let nombre = item.nombre_modelo.clone();
            let clave = if item.desplazamiento_blob > 0 {
                format!("@{}", item.desplazamiento_blob)
            } else {
                nombre.clone()
            };
            let subtexto = format!(
                "{} ({} bandas {})",
                item.nombre_modelo,
                item.numero_bandas,
                if item.es_parametrico {
                    "Parametrico PEQ"
                } else {
                    "Grafico"
                }
            );

            let fila = row![
                text(subtexto).size(12).width(Length::Fill),
                button(text("Cargar Perfil").size(11))
                    .on_press(MensajeUi::CargarPerfilAutoEq(clave)),
            ]
            .align_y(iced::Alignment::Center)
            .padding(4);

            lista_auriculares = lista_auriculares.push(fila);
        }

        let panel_autoeq = column![
            text("BASE DE DATOS AUTOEQ (+20,531 MODELOS DE AURICULARES)")
                .size(15)
                .color(COLOR_AMBAR_SUNLIGHT),
            row![entrada_busqueda, btn_buscar].spacing(8),
            scrollable(lista_auriculares).height(Length::Fixed(130.0)),
        ]
        .spacing(8);

        // 3. Seccion APO e Inyeccion de Audio en Windows & Dispositivos
        let estado_apo_txt = if self.estado_apo.apo_registrado {
            text("ESTADO: APO ACTIVO en audiodg.exe (El sonido de TODO Windows es procesado)")
                .size(13)
                .color(COLOR_AMBAR_SUNLIGHT)
        } else {
            text("ESTADO: APO NO REGISTRADO (Solo modo visualizador / render local o PipeWire Linux)")
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
            scrollable(lista_dispositivos).height(Length::Fixed(80.0)),
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

        column![panel_presets, space::vertical(), panel_autoeq, space::vertical(), panel_sistema]
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
            let alt_barra = if amplitud > 0.01 {
                (amplitud.clamp(0.0, 1.0) * (h * 0.85)).max(1.0)
            } else {
                0.0
            };
            if alt_barra <= 0.0 {
                continue;
            }
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
