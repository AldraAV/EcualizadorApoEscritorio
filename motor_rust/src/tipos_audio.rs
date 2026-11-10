//! Tipos de datos fundamentales y errores semanticos para el motor de audio DSP.
//! Cumple al 100% con la Soberania Linguistica del Español y Cero Emojis.

use std::fmt;
use serde::{Deserialize, Serialize};

/// Errores semanticos del motor de audio y decodificacion de presets.
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorMotorAudio {
    FrecuenciaMuestreoInvalida(f64),
    FrecuenciaFueraDeRango {
        frecuencia_hz: f64,
        nyquist_hz: f64,
    },
    FactorQInvalido(f64),
    InestabilidadNumericaBiquad,
    FormatoPresetInvalido(String),
    ArchivoNoEncontrado(String),
    ErrorEntradaSalida(String),
}

impl fmt::Display for ErrorMotorAudio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FrecuenciaMuestreoInvalida(fs) => {
                write!(f, "La frecuencia de muestreo debe ser mayor que cero (recibido: {} Hz)", fs)
            }
            Self::FrecuenciaFueraDeRango {
                frecuencia_hz,
                nyquist_hz,
            } => {
                write!(
                    f,
                    "La frecuencia de la banda ({} Hz) supera o iguala el limite de Nyquist ({} Hz)",
                    frecuencia_hz, nyquist_hz
                )
            }
            Self::FactorQInvalido(q) => {
                write!(f, "El factor Q de ancho de banda debe ser mayor que cero (recibido: {})", q)
            }
            Self::InestabilidadNumericaBiquad => {
                write!(f, "Inestabilidad numerica detectada en el denominador a0 del filtro biquad")
            }
            Self::FormatoPresetInvalido(detalle) => {
                write!(f, "Formato de preset de Poweramp invalido: {}", detalle)
            }
            Self::ArchivoNoEncontrado(ruta) => {
                write!(f, "No se pudo encontrar o abrir el archivo: {}", ruta)
            }
            Self::ErrorEntradaSalida(detalle) => {
                write!(f, "Error de lectura/escritura del sistema: {}", detalle)
            }
        }
    }
}

impl std::error::Error for ErrorMotorAudio {}

/// Tipos de filtro soportados por Poweramp Equalizer (JSON y blobs binarios PaP\x01).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TipoFiltro {
    /// Filtro Paso Bajo (Low Pass 12 dB/oct) - Poweramp JSON type = 0 (cuando es filtro de corte)
    PasoBajo,
    /// Filtro Paso Alto (High Pass 12 dB/oct) - Poweramp JSON type = 1 (cuando es filtro de corte)
    PasoAlto,
    /// Filtro Paso Banda (Band Pass) - Poweramp JSON type = 2
    PasoBanda,
    /// Filtro Pico Parametrico (Peaking EQ) - Poweramp JSON type = 3, blob PaP = 0x7003 / 0x5002
    PicoParametrico,
    /// Filtro Estante de Graves (Low Shelf) - Poweramp JSON type = 4, blob PaP = 0x7004
    EstanteGraves,
    /// Filtro Estante de Agudos (High Shelf) - Poweramp JSON type = 5, blob PaP = 0x7005
    EstanteAgudos,
}

impl TipoFiltro {
    /// Convierte el codigo entero de un archivo JSON de Poweramp al enum soberano.
    /// Nota: En Poweramp JSON, las dos primeras bandas con `color: 0` y `type: 0` (90 Hz)
    /// y `type: 1` (10000 Hz) representan los estantes de tono Graves (Low Shelf) y Agudos (High Shelf).
    pub fn desde_codigo_json_poweramp(codigo: u32, color: i32, frecuencia_hz: f64) -> Self {
        match codigo {
            0 => {
                if color == 0 && frecuencia_hz <= 250.0 {
                    Self::EstanteGraves
                } else {
                    Self::PasoBajo
                }
            }
            1 => {
                if color == 0 && frecuencia_hz >= 2000.0 {
                    Self::EstanteAgudos
                } else {
                    Self::PasoAlto
                }
            }
            2 => Self::PasoBanda,
            3 => Self::PicoParametrico,
            4 => Self::EstanteGraves,
            5 => Self::EstanteAgudos,
            _ => Self::PicoParametrico,
        }
    }

    /// Convierte el codigo binario de 32 bits del blob `PaP\x01` de `auto_eq.db`.
    pub fn desde_codigo_blob_pap(codigo_blob: u32) -> Self {
        match codigo_blob & 0xFFFF {
            0x7000 => Self::PasoBajo,
            0x7001 => Self::PasoAlto,
            0x7002 => Self::PasoBanda,
            0x7003 | 0x5002 => Self::PicoParametrico,
            0x7004 => Self::EstanteGraves,
            0x7005 => Self::EstanteAgudos,
            _ => Self::PicoParametrico,
        }
    }

    /// Devuelve el nombre identificador en espanol para serializacion hacia la UI.
    pub fn nombre_espanol(&self) -> &'static str {
        match self {
            Self::PasoBajo => "PasoBajo",
            Self::PasoAlto => "PasoAlto",
            Self::PasoBanda => "PasoBanda",
            Self::PicoParametrico => "PicoParametrico",
            Self::EstanteGraves => "EstanteGraves",
            Self::EstanteAgudos => "EstanteAgudos",
        }
    }

    /// Construye el tipo de filtro a partir de su nombre en espanol.
    pub fn desde_nombre(nombre: &str) -> Self {
        match nombre.trim() {
            "PasoBajo" | "LP" | "LPF" => Self::PasoBajo,
            "PasoAlto" | "HP" | "HPF" => Self::PasoAlto,
            "PasoBanda" | "BP" | "BPF" => Self::PasoBanda,
            "EstanteGraves" | "LS" | "LowShelf" => Self::EstanteGraves,
            "EstanteAgudos" | "HS" | "HighShelf" => Self::EstanteAgudos,
            _ => Self::PicoParametrico,
        }
    }
}

/// Canales a los que se aplica una banda en Poweramp Equalizer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CanalAplicacion {
    EstereoAmbos = 0,
    SoloIzquierdo = 1,
    SoloDerecho = 2,
}

/// Definicion de una banda individual del ecualizador parametrico o grafico.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BandaParametrica {
    pub identificador: usize,
    pub frecuencia_hz: f64,
    pub factor_q: f64,
    pub ganancia_db: f64,
    pub tipo: TipoFiltro,
    pub canales: CanalAplicacion,
    pub activo: bool,
    pub color_argb: i32,
}

impl BandaParametrica {
    pub fn nueva(
        identificador: usize,
        frecuencia_hz: f64,
        factor_q: f64,
        ganancia_db: f64,
        tipo: TipoFiltro,
    ) -> Self {
        Self {
            identificador,
            frecuencia_hz,
            factor_q,
            ganancia_db,
            tipo,
            canales: CanalAplicacion::EstereoAmbos,
            activo: true,
            color_argb: -6543440,
        }
    }
}

/// Controles maestros de Tono estilo Poweramp (peq_equ_tone.xml).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ParametrosTono {
    pub tono_activo: bool,
    pub graves_ganancia_db: f64,
    pub graves_frecuencia_hz: f64,
    pub graves_factor_q: f64,
    pub agudos_ganancia_db: f64,
    pub agudos_frecuencia_hz: f64,
    pub agudos_factor_q: f64,
}

impl Default for ParametrosTono {
    fn default() -> Self {
        Self {
            tono_activo: false,
            graves_ganancia_db: 0.0,
            graves_frecuencia_hz: 90.0,
            graves_factor_q: 0.8,
            agudos_ganancia_db: 0.0,
            agudos_frecuencia_hz: 10000.0,
            agudos_factor_q: 0.6,
        }
    }
}

/// Controles de Espacio estilo Poweramp (Balance, Stereo Expand, Mono).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ParametrosEspacio {
    /// Balance L/R de -1.0 (100% Izquierda) a +1.0 (100% Derecha). 0.0 es centro.
    pub balance: f64,
    /// Expansion estereo Mid/Side de 0.0 (normal 100%) a 1.0 (200% ancho estereo).
    pub expansion_estereo: f64,
    /// Mezcla monofonica activa.
    pub modo_mono: bool,
}

impl Default for ParametrosEspacio {
    fn default() -> Self {
        Self {
            balance: 0.0,
            expansion_estereo: 0.0,
            modo_mono: false,
        }
    }
}

/// Controles del Compresor Dinamico estilo Poweramp (peq_compr.xml).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ParametrosCompresor {
    pub activo: bool,
    /// Umbral / Limite en dB (-60.0 a 0.0 dB).
    pub limite_db: f64,
    /// Proporcion / Ratio de compresion (1.0 a 20.0).
    pub proporcion: f64,
    /// Tiempo de ataque en milisegundos (0.1 a 200.0 ms).
    pub ataque_ms: f64,
    /// Tiempo de lanzamiento/liberacion en milisegundos (10.0 a 1000.0 ms).
    pub lanzamiento_ms: f64,
    /// Frecuencia del filtro de cadena lateral (sidechain high-pass/shelf) en Hz (20.0 a 2000.0 Hz).
    pub frecuencia_sidechain_hz: f64,
    /// Ganancia de compensacion (Makeup Gain) en dB (-12.0 a +24.0 dB).
    pub ganancia_maquillaje_db: f64,
}

impl Default for ParametrosCompresor {
    fn default() -> Self {
        Self {
            activo: false,
            limite_db: 0.0,
            proporcion: 2.5,
            ataque_ms: 10.0,
            lanzamiento_ms: 100.0,
            frecuencia_sidechain_hz: 90.0,
            ganancia_maquillaje_db: 0.0,
        }
    }
}

/// Controles del Procesador de Reverberacion estilo Poweramp (Tamano, Preretardo, Amortiguacion, Filtro, Mezclado).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ParametrosReverberacion {
    pub activo: bool,
    /// Tamano de la sala (0.0 a 1.0).
    pub tamano: f64,
    /// Preretardo en milisegundos (0.0 a 100.0 ms).
    pub preretardo_ms: f64,
    /// Amortiguacion de altas frecuencias en el lazo de reflexion (0.0 a 1.0).
    pub amortiguacion: f64,
    /// Filtro tonal / fundido de cola (0.0 = calido/oscuro, 0.5 = neutro, 1.0 = brillante).
    pub filtro_tonal: f64,
    /// Mezclado Dry/Wet (0.0 = 100% seco, 1.0 = 100% humedo).
    pub mezclado: f64,
}

impl Default for ParametrosReverberacion {
    fn default() -> Self {
        Self {
            activo: false,
            tamano: 0.5,
            preretardo_ms: 15.0,
            amortiguacion: 0.5,
            filtro_tonal: 0.5,
            mezclado: 0.0,
        }
    }
}

/// Estado integral de toda la cadena DSP del ecualizador.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EstadoCompletoDsp {
    pub nombre_perfil: String,
    pub ecualizador_activo: bool,
    pub modo_parametrico: bool,
    pub frecuencia_muestreo_hz: f64,
    pub ganancia_preamp_db: f64,
    pub limitador_activo: bool,
    pub bandas: Vec<BandaParametrica>,
    pub tono: ParametrosTono,
    pub espacio: ParametrosEspacio,
    pub compresor: ParametrosCompresor,
    pub reverberacion: ParametrosReverberacion,
}

impl Default for EstadoCompletoDsp {
    fn default() -> Self {
        let frecuencias_iso = [
            (31.5, TipoFiltro::EstanteGraves),
            (63.0, TipoFiltro::PicoParametrico),
            (125.0, TipoFiltro::PicoParametrico),
            (250.0, TipoFiltro::PicoParametrico),
            (500.0, TipoFiltro::PicoParametrico),
            (1000.0, TipoFiltro::PicoParametrico),
            (2000.0, TipoFiltro::PicoParametrico),
            (4000.0, TipoFiltro::PicoParametrico),
            (8000.0, TipoFiltro::PicoParametrico),
            (16000.0, TipoFiltro::EstanteAgudos),
        ];

        let bandas = frecuencias_iso
            .iter()
            .enumerate()
            .map(|(indice, &(frecuencia_hz, tipo))| {
                BandaParametrica::nueva(indice + 1, frecuencia_hz, 1.41, 0.0, tipo)
            })
            .collect();

        Self {
            nombre_perfil: String::from("Sonido Original (Plano)"),
            ecualizador_activo: true,
            modo_parametrico: true,
            frecuencia_muestreo_hz: 48000.0,
            ganancia_preamp_db: 0.0,
            limitador_activo: true,
            bandas,
            tono: ParametrosTono::default(),
            espacio: ParametrosEspacio::default(),
            compresor: ParametrosCompresor::default(),
            reverberacion: ParametrosReverberacion::default(),
        }
    }
}
