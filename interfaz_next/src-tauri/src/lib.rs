//! Biblioteca y comandos IPC de Tauri v2 para el Ecualizador Poweramp Soberano.
//! Implementado al 100% en Espanol Soberano, sin emojis en codigo fuente.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::State;

use motor_ecualizador_poweramp::cadena_maestra_dsp::CadenaMaestraDsp;
use motor_ecualizador_poweramp::captura_wasapi_loopback::{
    enumerar_dispositivos_salida_windows, DispositivoAudioWindows,
};
use motor_ecualizador_poweramp::cargador_presets::{
    buscar_presets_autoeq_en_archivo, cargar_preset_autoeq_por_consulta, exportar_archivo_apo,
    generar_configuracion_apo, ResumenPresetAutoEq,
};
use motor_ecualizador_poweramp::tipos_audio::EstadoCompletoDsp;

/// Frecuencias logaritmicas precalculadas (128 puntos) de 20 Hz a 20 kHz para el trazado de la curva Biquad.
const PUNTOS_CURVA_RESPUESTA: usize = 128;

fn generar_frecuencias_curva() -> Vec<f64> {
    let mut frecs = Vec::with_capacity(PUNTOS_CURVA_RESPUESTA);
    for i in 0..PUNTOS_CURVA_RESPUESTA {
        let proporcion = i as f64 / (PUNTOS_CURVA_RESPUESTA - 1) as f64;
        let frecuencia = 20.0 * (1000.0_f64).powf(proporcion);
        frecs.push(frecuencia);
    }
    frecs
}

/// Paquete de telemetria en tiempo real enviado a la interfaz grafica.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetriaUi {
    pub barras: Vec<f32>,
    pub reduccion_compresor_db: f64,
    pub pico_salida_db: f64,
    pub puntos_curva: Vec<f64>,
}

/// Metadatos de un archivo de audio cacheado en memoria para ser procesado por el backend DSP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioCacheado {
    pub nombre_archivo: String,
    pub frecuencia_muestreo_hz: u32,
    pub canales: u16,
    pub muestras: Vec<f32>,
    pub tamaño_bytes: usize,
}

/// Resultado de aplicar la cadena DSP a un archivo de audio cacheado en memoria.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioProcesado {
    pub nombre_archivo: String,
    pub frecuencia_muestreo_hz: u32,
    pub canales: u16,
    pub muestras_procesadas: Vec<f32>,
    pub muestras_totales: usize,
    pub pico_salida_db: f64,
}

/// Estado global persistente administrado por Tauri.
pub struct EstadoAplicacionTauri {
    pub cadena_dsp: Mutex<CadenaMaestraDsp>,
    pub audio_cache: Mutex<Option<AudioCacheado>>,
    pub id_dispositivo_seleccionado: Mutex<Option<String>>,
    pub ruta_autoeq_db: PathBuf,
    pub ruta_configuracion_apo: PathBuf,
    pub frecuencias_curva: Vec<f64>,
}

fn resolver_ruta_autoeq() -> PathBuf {
    let rutas_posibles = [
        PathBuf::from("../../recursos/auto_eq.db"),
        PathBuf::from("../recursos/auto_eq.db"),
        PathBuf::from("recursos/auto_eq.db"),
        PathBuf::from("auto_eq.db"),
    ];

    for ruta in &rutas_posibles {
        if ruta.exists() {
            return ruta.clone();
        }
    }

    PathBuf::from("../../recursos/auto_eq.db")
}

fn leer_wav_a_muestras(contenido: &[u8]) -> Result<(u32, u16, Vec<f32>), String> {
    if contenido.len() < 12 || &contenido[..4] != b"RIFF" || &contenido[8..12] != b"WAVE" {
        return Err("El archivo no tiene formato WAV valido".to_string());
    }

    let mut offset = 12usize;
    let mut frecuencia_muestreo_hz: u32 = 44100;
    let mut canales: u16 = 2;
    let mut bits_por_muestra: u16 = 16;
    let mut datos: Option<&[u8]> = None;

    while offset + 8 <= contenido.len() {
        let chunk_id = &contenido[offset..offset + 4];
        let chunk_size = u32::from_le_bytes(
            contenido[offset + 4..offset + 8]
                .try_into()
                .map_err(|_| "Cabecera WAV corrupta".to_string())?,
        ) as usize;
        let chunk_data_inicio = offset + 8;
        let chunk_data_fin = chunk_data_inicio
            .checked_add(chunk_size)
            .ok_or_else(|| "Tamano de chunk WAV invalido".to_string())?;

        if chunk_data_fin > contenido.len() {
            return Err("El archivo WAV termina antes de completar el chunk".to_string());
        }

        match chunk_id {
            b"fmt " => {
                if chunk_size < 16 {
                    return Err("El chunk fmt del WAV es demasiado corto".to_string());
                }
                let formato = u16::from_le_bytes(
                    contenido[chunk_data_inicio..chunk_data_inicio + 2]
                        .try_into()
                        .map_err(|_| "Formato WAV invalido".to_string())?,
                );
                if formato != 1 && formato != 3 {
                    return Err(format!("Formato WAV no soportado: {}", formato));
                }
                canales = u16::from_le_bytes(
                    contenido[chunk_data_inicio + 2..chunk_data_inicio + 4]
                        .try_into()
                        .map_err(|_| "Canales WAV invalidos".to_string())?,
                );
                frecuencia_muestreo_hz = u32::from_le_bytes(
                    contenido[chunk_data_inicio + 4..chunk_data_inicio + 8]
                        .try_into()
                        .map_err(|_| "Frecuencia de muestreo WAV invalida".to_string())?,
                );
                bits_por_muestra = u16::from_le_bytes(
                    contenido[chunk_data_inicio + 14..chunk_data_inicio + 16]
                        .try_into()
                        .map_err(|_| "Bits por muestra WAV invalidos".to_string())?,
                );
            }
            b"data" => {
                datos = Some(&contenido[chunk_data_inicio..chunk_data_fin]);
                break;
            }
            _ => {}
        }

        offset = chunk_data_fin + (chunk_size % 2);
    }

    let data = datos.ok_or_else(|| "No se encontro el chunk data dentro del WAV".to_string())?;
    if data.is_empty() {
        return Ok((frecuencia_muestreo_hz, canales, Vec::new()));
    }

    let bytes_por_muestra = (bits_por_muestra as usize).max(1) / 8;
    let mut muestras = Vec::with_capacity(data.len() / bytes_por_muestra.max(1));

    for bloque in data.chunks_exact(bytes_por_muestra.max(1)) {
        let valor = match bits_por_muestra {
            8 => {
                let codigo = bloque[0] as i16 - 128;
                (codigo as f32) / 128.0
            }
            16 => i16::from_le_bytes([bloque[0], bloque[1]]) as f32 / 32768.0,
            24 => {
                let entero = i32::from_le_bytes([bloque[0], bloque[1], bloque[2], 0]);
                (entero as f32) / 8_388_608.0
            }
            32 => f32::from_le_bytes([bloque[0], bloque[1], bloque[2], bloque[3]]),
            _ => return Err(format!("Bits por muestra no soportados: {}", bits_por_muestra)),
        };
        muestras.push(valor.clamp(-1.0, 1.0));
    }

    Ok((frecuencia_muestreo_hz, canales, muestras))
}

// ============================================================================
// MODULO DE COMANDOS TAURI
// ============================================================================

mod comandos {
    use super::*;

    /// Obtiene una copia completa del estado actual de la cadena DSP.
    #[tauri::command]
    pub fn obtener_estado_dsp(
        estado_app: State<'_, EstadoAplicacionTauri>,
    ) -> Result<EstadoCompletoDsp, String> {
        let cadena = estado_app
            .cadena_dsp
            .lock()
            .map_err(|e| format!("Error al bloquear el mutex de la cadena DSP: {}", e))?;
        Ok(cadena.obtener_estado().clone())
    }

    /// Guarda un archivo WAV o PCM en cache del backend para aplicarle la cadena DSP.
    #[tauri::command]
    pub fn cargar_audio_en_cache(
        nombre_archivo: String,
        contenido: Vec<u8>,
        estado_app: State<'_, EstadoAplicacionTauri>,
    ) -> Result<AudioCacheado, String> {
        if nombre_archivo.trim().is_empty() {
            return Err("El nombre del archivo de audio es obligatorio".to_string());
        }
        if contenido.is_empty() {
            return Err("El archivo de audio llega vacio".to_string());
        }

        let (frecuencia_muestreo_hz, canales, muestras) = leer_wav_a_muestras(&contenido)?;
        let audio_cacheado = AudioCacheado {
            nombre_archivo,
            frecuencia_muestreo_hz,
            canales,
            muestras,
            tamaño_bytes: contenido.len(),
        };

        let mut audio_guardado = estado_app
            .audio_cache
            .lock()
            .map_err(|e| format!("Error al bloquear el cache de audio: {}", e))?;
        *audio_guardado = Some(audio_cacheado.clone());
        Ok(audio_cacheado)
    }

    /// Devuelve el estado del cache de audio actual si existe.
    #[tauri::command]
    pub fn obtener_cache_audio(
        estado_app: State<'_, EstadoAplicacionTauri>,
    ) -> Result<Option<AudioCacheado>, String> {
        let audio_guardado = estado_app
            .audio_cache
            .lock()
            .map_err(|e| format!("Error al consultar el cache de audio: {}", e))?;
        Ok(audio_guardado.clone())
    }

    /// Procesa el audio cacheado con la cadena DSP del backend y devuelve el resultado en memoria.
    #[tauri::command]
    pub fn ecualizar_audio_en_cache(
        estado_app: State<'_, EstadoAplicacionTauri>,
    ) -> Result<AudioProcesado, String> {
        let audio_guardado = estado_app
            .audio_cache
            .lock()
            .map_err(|e| format!("Error al bloquear el cache de audio: {}", e))?;
        let audio = audio_guardado
            .clone()
            .ok_or_else(|| "No hay audio cargado en el cache del backend".to_string())?;
        drop(audio_guardado);

        let mut cadena = estado_app
            .cadena_dsp
            .lock()
            .map_err(|e| format!("Error al bloquear la cadena DSP para procesar audio: {}", e))?;

        let mut muestras_procesadas = Vec::with_capacity(audio.muestras.len());
        if audio.muestras.is_empty() {
            return Ok(AudioProcesado {
                nombre_archivo: audio.nombre_archivo,
                frecuencia_muestreo_hz: audio.frecuencia_muestreo_hz,
                canales: audio.canales,
                muestras_procesadas: Vec::new(),
                muestras_totales: 0,
                pico_salida_db: cadena.obtener_pico_salida_db(),
            });
        }

        if audio.canales == 1 {
            for muestra in audio.muestras {
                let (izq, der) = cadena.procesar_par_estereo(muestra, muestra);
                muestras_procesadas.push(izq);
                muestras_procesadas.push(der);
            }
        } else if audio.muestras.len() % 2 == 0 {
            for par in audio.muestras.chunks_exact(2) {
                let (izq, der) = cadena.procesar_par_estereo(par[0], par[1]);
                muestras_procesadas.push(izq);
                muestras_procesadas.push(der);
            }
        } else {
            return Err("El audio cacheado no tiene un numero par de muestras para stereo".to_string());
        }

        let pico_salida_db = cadena.obtener_pico_salida_db();
        Ok(AudioProcesado {
            nombre_archivo: audio.nombre_archivo,
            frecuencia_muestreo_hz: audio.frecuencia_muestreo_hz,
            canales: audio.canales,
            muestras_procesadas: muestras_procesadas.clone(),
            muestras_totales: muestras_procesadas.len(),
            pico_salida_db,
        })
    }

    /// Aplica un nuevo estado a la cadena DSP y sincroniza con el archivo Equalizer APO.
    #[tauri::command]
    pub fn actualizar_parametros_dsp(
        nuevo_estado: EstadoCompletoDsp,
        estado_app: State<'_, EstadoAplicacionTauri>,
    ) -> Result<(), String> {
        {
            let mut cadena = estado_app
                .cadena_dsp
                .lock()
                .map_err(|e| format!("Error al bloquear el mutex de la cadena DSP: {}", e))?;
            cadena
                .aplicar_estado(nuevo_estado.clone())
                .map_err(|e| format!("Fallo al aplicar nuevo estado DSP: {}", e))?;
        }

        let _ = exportar_archivo_apo(&estado_app.ruta_configuracion_apo, &nuevo_estado);
        Ok(())
    }

    /// Obtiene la telemetria viva de las barras FFT de 64 bandas, compresor y curva calculada.
    #[tauri::command]
    pub fn obtener_telemetria_espectro(
        estado_app: State<'_, EstadoAplicacionTauri>,
    ) -> Result<TelemetriaUi, String> {
        let cadena = estado_app
            .cadena_dsp
            .lock()
            .map_err(|e| format!("Error al bloquear mutex DSP: {}", e))?;

        let barras_array = cadena.obtener_barras_espectro();
        let reduccion_compresor_db = cadena.obtener_reduccion_compresor_db();
        let pico_salida_db = cadena.obtener_pico_salida_db();
        let puntos_curva = cadena.evaluar_curva_respuesta_db(&estado_app.frecuencias_curva);

        Ok(TelemetriaUi {
            barras: barras_array.to_vec(),
            reduccion_compresor_db,
            pico_salida_db,
            puntos_curva,
        })
    }

    /// Busca perfiles de auriculares en la base de datos de 5,400 modelos de AutoEQ.
    #[tauri::command]
    pub fn buscar_auriculares_autoeq(
        consulta: String,
        estado_app: State<'_, EstadoAplicacionTauri>,
    ) -> Result<Vec<ResumenPresetAutoEq>, String> {
        if !estado_app.ruta_autoeq_db.exists() {
            return Err(format!(
                "No se encontro el archivo auto_eq.db en: {}",
                estado_app.ruta_autoeq_db.display()
            ));
        }

        buscar_presets_autoeq_en_archivo(&estado_app.ruta_autoeq_db, &consulta, 30)
            .map_err(|e| format!("Error al buscar en auto_eq.db: {}", e))
    }

    /// Carga un perfil AutoEQ por desplazamiento de blob y lo aplica a la cadena activa.
    #[tauri::command]
    pub fn cargar_perfil_autoeq(
        desplazamiento_blob: usize,
        _nombre_modelo: String,
        estado_app: State<'_, EstadoAplicacionTauri>,
    ) -> Result<EstadoCompletoDsp, String> {
        let consulta = format!("@{}", desplazamiento_blob);
        let mut nuevo_estado =
            cargar_preset_autoeq_por_consulta(&estado_app.ruta_autoeq_db, &consulta)
                .map_err(|e| format!("Fallo al cargar perfil desde auto_eq.db: {}", e))?;

        {
            let mut cadena = estado_app
                .cadena_dsp
                .lock()
                .map_err(|e| format!("Error al bloquear mutex DSP: {}", e))?;
            nuevo_estado.frecuencia_muestreo_hz = cadena.obtener_estado().frecuencia_muestreo_hz;
            cadena
                .aplicar_estado(nuevo_estado.clone())
                .map_err(|e| format!("Error al aplicar perfil en DSP: {}", e))?;
        }

        let _ = exportar_archivo_apo(&estado_app.ruta_configuracion_apo, &nuevo_estado);
        Ok(nuevo_estado)
    }

    /// Enumera los dispositivos fisicos de salida de audio activos en Windows.
    #[tauri::command]
    pub fn listar_dispositivos_audio() -> Result<Vec<DispositivoAudioWindows>, String> {
        enumerar_dispositivos_salida_windows()
    }

    /// Selecciona el dispositivo de audio preferido para monitoreo / loopback.
    #[tauri::command]
    pub fn seleccionar_dispositivo_salida(
        id_dispositivo: String,
        estado_app: State<'_, EstadoAplicacionTauri>,
    ) -> Result<(), String> {
        let mut id_guardado = estado_app
            .id_dispositivo_seleccionado
            .lock()
            .map_err(|e| format!("Error al bloquear mutex: {}", e))?;
        *id_guardado = Some(id_dispositivo);
        Ok(())
    }

    /// Exporta el archivo de configuracion en formato texto para Equalizer APO.
    #[tauri::command]
    pub fn exportar_archivo_apo_config(
        estado_app: State<'_, EstadoAplicacionTauri>,
    ) -> Result<String, String> {
        let cadena = estado_app
            .cadena_dsp
            .lock()
            .map_err(|e| format!("Error al bloquear mutex: {}", e))?;
        let estado = cadena.obtener_estado();
        let texto = generar_configuracion_apo(estado);
        let _ = exportar_archivo_apo(&estado_app.ruta_configuracion_apo, estado);
        Ok(texto)
    }
}

// ============================================================================
// INICIALIZACION TAURI
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let ruta_autoeq_db = resolver_ruta_autoeq();
    let ruta_configuracion_apo = PathBuf::from("apo_config.txt");

    let estado_inicial = EstadoAplicacionTauri {
        cadena_dsp: Mutex::new(CadenaMaestraDsp::nueva(48000.0)),
        audio_cache: Mutex::new(None),
        id_dispositivo_seleccionado: Mutex::new(None),
        ruta_autoeq_db,
        ruta_configuracion_apo,
        frecuencias_curva: generar_frecuencias_curva(),
    };

    tauri::Builder::default()
        .manage(estado_inicial)
        .invoke_handler(tauri::generate_handler![
            comandos::obtener_estado_dsp,
            comandos::cargar_audio_en_cache,
            comandos::obtener_cache_audio,
            comandos::ecualizar_audio_en_cache,
            comandos::actualizar_parametros_dsp,
            comandos::obtener_telemetria_espectro,
            comandos::buscar_auriculares_autoeq,
            comandos::cargar_perfil_autoeq,
            comandos::listar_dispositivos_audio,
            comandos::seleccionar_dispositivo_salida,
            comandos::exportar_archivo_apo_config,
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error fatal al iniciar la aplicacion tauri");
}
