//! enrutador_pipewire.rs — Administrador Soberano de Nodos y Streams de Audio PipeWire en Linux.
//! Detecta en tiempo real que aplicaciones o reproductores estan emitiendo sonido
//! (Spotify, Firefox, VLC, Chromium, etc.) y permite enrutar selectivamente
//! solo los flujos elegidos hacia la cadena maestra DSP del ecualizador.
//!
//! 100% en espanol soberano y sin emojis en codigo fuente.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
#[cfg(target_os = "linux")]
use std::process::Command;

/// Informacion estructurada de una aplicacion o reproductor emitiendo audio en PipeWire.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReproductorAudioPipeWire {
    /// Identificador numerico del nodo en el grafo de PipeWire.
    pub identificador_nodo: u32,
    /// Nombre amigable de la aplicacion (ej. "Spotify", "firefox", "VLC media player").
    pub nombre_aplicacion: String,
    /// Descripcion del flujo o medio actual (ej. "Playback", titulo de pista o stream).
    pub nombre_medio: String,
    /// Nombre interno del nodo en PipeWire.
    pub nombre_nodo: String,
    /// Identificador del proceso emisor (PID) si esta disponible.
    pub pid_proceso: Option<u32>,
    /// Si el flujo de audio esta actualmente activo/reproduciendo muestras.
    pub esta_reproduciendo: bool,
    /// Si el usuario ha seleccionado ecualizar este reproductor en especifico.
    pub ecualizacion_habilitada: bool,
}

impl ReproductorAudioPipeWire {
    /// Retorna una etiqueta formateada para mostrar en la interfaz de usuario.
    pub fn etiqueta_visual(&self) -> String {
        if !self.nombre_medio.is_empty() && self.nombre_medio != self.nombre_aplicacion {
            format!("{} ({})", self.nombre_aplicacion, self.nombre_medio)
        } else {
            self.nombre_aplicacion.clone()
        }
    }
}

/// Estado global del enrutador y lista de aplicaciones controladas.
#[derive(Debug, Clone, Default)]
pub struct EstadoEnrutadorPipeWire {
    /// Reproductores actualmente detectados en el sistema.
    pub reproductores: Vec<ReproductorAudioPipeWire>,
    /// Conjunto de nombres de aplicaciones que tienen el ecualizador activado.
    pub aplicaciones_habilitadas: HashSet<String>,
    /// Si la captura selectiva por PipeWire esta activa a nivel global.
    pub modo_selectivo_activo: bool,
    /// Nombre del nodo virtual o sink creado por el ecualizador en PipeWire.
    pub nombre_sink_virtual: String,
}

impl EstadoEnrutadorPipeWire {
    pub fn nuevo() -> Self {
        let mut apps = HashSet::new();
        // Habilitar por defecto los reproductores de musica mas populares si existen
        apps.insert("Spotify".to_string());
        apps.insert("spotify".to_string());
        apps.insert("firefox".to_string());
        apps.insert("Chromium".to_string());

        Self {
            reproductores: Vec::new(),
            aplicaciones_habilitadas: apps,
            modo_selectivo_activo: true,
            nombre_sink_virtual: "AldraPowerampEqualizerSink".to_string(),
        }
    }

    /// Actualiza la lista de reproductores detectados preservando la seleccion de ecualizacion.
    pub fn sincronizar_con_sistema(&mut self) {
        let lista_sistema = enumerar_reproductores_pipewire().unwrap_or_default();
        let mut lista_actualizada = Vec::with_capacity(lista_sistema.len());

        for mut rep in lista_sistema {
            // Un reproductor se considera habilitado si su nombre esta en el conjunto
            let esta_habilitado = self.aplicaciones_habilitadas.contains(&rep.nombre_aplicacion);
            rep.ecualizacion_habilitada = esta_habilitado;
            lista_actualizada.push(rep);
        }

        self.reproductores = lista_actualizada;
    }

    /// Alterna el estado de ecualizacion para un reproductor especifico.
    pub fn conmutar_reproductor(&mut self, nombre_aplicacion: &str, habilitar: bool) {
        if habilitar {
            self.aplicaciones_habilitadas.insert(nombre_aplicacion.to_string());
        } else {
            self.aplicaciones_habilitadas.remove(nombre_aplicacion);
        }

        for rep in &mut self.reproductores {
            if rep.nombre_aplicacion == nombre_aplicacion {
                rep.ecualizacion_habilitada = habilitar;
            }
        }

        // Si estamos en un sistema con PipeWire activo, aplicar el enlace
        let _ = aplicar_enrutamiento_sistema(
            nombre_aplicacion,
            habilitar,
            &self.nombre_sink_virtual,
        );
    }

    /// Aplica en bloque el enrutamiento para todos los reproductores detectados.
    pub fn aplicar_enrutamiento_completo(&self) {
        for rep in &self.reproductores {
            let habilitar = if self.modo_selectivo_activo {
                rep.ecualizacion_habilitada
            } else {
                true
            };
            let _ = aplicar_enrutamiento_sistema(
                &rep.nombre_aplicacion,
                habilitar,
                &self.nombre_sink_virtual,
            );
        }
    }
}

/// Verifica si el subsistema PipeWire esta presente y respondiendo en el sistema anfitrion.
pub fn verificar_pipewire_disponible() -> bool {
    #[cfg(target_os = "linux")]
    {
        // 1. Verificar existencia del comando pw-dump
        if let Ok(salida) = Command::new("pw-dump").arg("-v").output() {
            if salida.status.success() {
                return true;
            }
        }
        // 2. Verificar socket de PipeWire en runtime directory
        if let Ok(dir_runtime) = std::env::var("XDG_RUNTIME_DIR") {
            let ruta_socket = std::path::Path::new(&dir_runtime).join("pipewire-0");
            if ruta_socket.exists() {
                return true;
            }
        }
        false
    }

    #[cfg(not(target_os = "linux"))]
    {
        // En Windows / macOS no hay PipeWire nativo
        false
    }
}

/// Enumera todos los nodos de tipo `Stream/Output/Audio` activos en el grafo de PipeWire.
pub fn enumerar_reproductores_pipewire() -> Result<Vec<ReproductorAudioPipeWire>, String> {
    #[cfg(target_os = "linux")]
    {
        let salida = Command::new("pw-dump")
            .arg("Node")
            .output()
            .map_err(|e| format!("No se pudo ejecutar pw-dump en Linux: {}", e))?;

        if !salida.status.success() {
            return Err(format!(
                "pw-dump retorno codigo de error {}",
                salida.status.code().unwrap_or(-1)
            ));
        }

        let texto_json = String::from_utf8_lossy(&salida.stdout);
        parsear_nodos_pipewire_json(&texto_json)
    }

    #[cfg(not(target_os = "linux"))]
    {
        // Proveer lista demostrativa de referencia cuando se compile o pruebe en Windows
        Ok(vec![
            ReproductorAudioPipeWire {
                identificador_nodo: 42,
                nombre_aplicacion: "Spotify".to_string(),
                nombre_medio: "Musica en streaming".to_string(),
                nombre_nodo: "spotify_playback_stream".to_string(),
                pid_proceso: Some(1337),
                esta_reproduciendo: true,
                ecualizacion_habilitada: true,
            },
            ReproductorAudioPipeWire {
                identificador_nodo: 58,
                nombre_aplicacion: "firefox".to_string(),
                nombre_medio: "YouTube Video Audio".to_string(),
                nombre_nodo: "firefox_media_stream".to_string(),
                pid_proceso: Some(2048),
                esta_reproduciendo: true,
                ecualizacion_habilitada: true,
            },
            ReproductorAudioPipeWire {
                identificador_nodo: 73,
                nombre_aplicacion: "VLC media player".to_string(),
                nombre_medio: "Pista FLAC de alta resolucion".to_string(),
                nombre_nodo: "vlc_audio_out".to_string(),
                pid_proceso: Some(4096),
                esta_reproduciendo: false,
                ecualizacion_habilitada: false,
            },
            ReproductorAudioPipeWire {
                identificador_nodo: 91,
                nombre_aplicacion: "Discord".to_string(),
                nombre_medio: "Canal de voz (Bypass)".to_string(),
                nombre_nodo: "discord_voice_output".to_string(),
                pid_proceso: Some(8192),
                esta_reproduciendo: false,
                ecualizacion_habilitada: false,
            },
        ])
    }
}

/// Parsea la salida JSON de `pw-dump Node` extrayendo los streams de reproduccion de aplicaciones.
pub fn parsear_nodos_pipewire_json(texto_json: &str) -> Result<Vec<ReproductorAudioPipeWire>, String> {
    let valor_json: serde_json::Value = serde_json::from_str(texto_json)
        .map_err(|e| format!("Error deserializando JSON de PipeWire: {}", e))?;

    let mut reproductores = Vec::new();
    let arreglo = valor_json
        .as_array()
        .ok_or_else(|| "La salida de pw-dump no es un arreglo JSON".to_string())?;

    for item in arreglo {
        let info = match item.get("info") {
            Some(i) => i,
            None => continue,
        };

        let props = match info.get("props") {
            Some(p) => p,
            None => continue,
        };

        // Filtrar nodos cuya clase de medio sea "Stream/Output/Audio" (aplicaciones reproduciendo)
        let clase_medio = props
            .get("media.class")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if clase_medio != "Stream/Output/Audio" {
            continue;
        }

        let identificador = item.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        let nombre_app = props
            .get("application.name")
            .or_else(|| props.get("app.name"))
            .or_else(|| props.get("node.name"))
            .and_then(|v| v.as_str())
            .unwrap_or("Reproductor Desconocido")
            .to_string();

        let nombre_medio = props
            .get("media.name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let nombre_nodo = props
            .get("node.name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let pid = props
            .get("application.process.id")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        let estado_str = info
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("idle");
        let esta_reproduciendo = estado_str == "running";

        reproductores.push(ReproductorAudioPipeWire {
            identificador_nodo: identificador,
            nombre_aplicacion: nombre_app,
            nombre_medio,
            nombre_nodo,
            pid_proceso: pid,
            esta_reproduciendo,
            ecualizacion_habilitada: true,
        });
    }

    Ok(reproductores)
}

/// Aplica el enlace o desvinculacion del reproductor hacia el sink virtual de PipeWire.
pub fn aplicar_enrutamiento_sistema(
    nombre_aplicacion: &str,
    habilitar: bool,
    _nombre_sink_virtual: &str,
) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        // En Linux, usar pw-link para conectar/desconectar los puertos del stream
        let accion = if habilitar { "vincular" } else { "desvincular" };
        let flag_desvincular = if habilitar { "" } else { "-d" };

        let mut comando = Command::new("pw-link");
        if !habilitar {
            comando.arg(flag_desvincular);
        }
        comando.arg(format!("{}:output_FL", nombre_aplicacion));
        comando.arg(format!("{}:playback_FL", _nombre_sink_virtual));

        let _ = comando.output();
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (nombre_aplicacion, habilitar);
        Ok(())
    }
}
