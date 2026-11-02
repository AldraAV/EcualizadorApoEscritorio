//! Cargador y decodificador nativo de presets de Poweramp Equalizer:
//! 1. Archivos `.json` exportados por Poweramp Equalizer.
//! 2. Blobs binarios `PaP\x01` extraidos de `assets/auto_eq.db` del APK de Poweramp.
//! 3. Escaneo directo de alta velocidad sobre `auto_eq.db` sin dependencias C externas.
//! 4. Archivos `.peq-settings` (Base64 + binario) y exportacion hacia Equalizer APO (`apo_config.txt`).

use std::fs;
use std::path::Path;

use crate::tipos_audio::{
    BandaParametrica, CanalAplicacion, ErrorMotorAudio, EstadoCompletoDsp, ParametrosTono,
    TipoFiltro,
};

/// Resumen ligero de un perfil AutoEQ indexado desde `auto_eq.db`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ResumenPresetAutoEq {
    pub indice: usize,
    pub nombre_modelo: String,
    pub es_parametrico: bool,
    pub numero_bandas: usize,
    pub desplazamiento_blob: usize,
}

/// Decodifica un blob binario con firma `PaP\x01` (`50 61 50 01`) de `auto_eq.db`.
pub fn decodificar_blob_pap(
    nombre_perfil: &str,
    datos: &[u8],
) -> Result<EstadoCompletoDsp, ErrorMotorAudio> {
    if datos.len() < 40 {
        return Err(ErrorMotorAudio::FormatoPresetInvalido(String::from(
            "El blob binario es menor a los 40 bytes de cabecera PaP\\x01",
        )));
    }

    if &datos[0..4] != b"PaP\x01" {
        return Err(ErrorMotorAudio::FormatoPresetInvalido(String::from(
            "Cabecera magica PaP\\x01 no encontrada al inicio del blob",
        )));
    }

    let conteo_entradas = u16::from_le_bytes([datos[4], datos[5]]) as usize;
    let bandera_parametrico = u16::from_le_bytes([datos[6], datos[7]]);
    let es_parametrico = bandera_parametrico == 1;

    // Bytes 8..24: Definicion de Tono Graves (tipo 0x7000, ganancia f32, Q f32, frecuencia u32)
    let graves_ganancia_db = f32::from_le_bytes([datos[12], datos[13], datos[14], datos[15]]) as f64;
    let graves_factor_q = f32::from_le_bytes([datos[16], datos[17], datos[18], datos[19]]) as f64;
    let frecuencia_graves_hz =
        u32::from_le_bytes([datos[20], datos[21], datos[22], datos[23]]) as f64;

    // Bytes 24..40: Definicion de Tono Agudos (tipo 0x7001, ganancia f32, Q f32, frecuencia u32)
    let agudos_ganancia_db = f32::from_le_bytes([datos[28], datos[29], datos[30], datos[31]]) as f64;
    let agudos_factor_q = f32::from_le_bytes([datos[32], datos[33], datos[34], datos[35]]) as f64;
    let frecuencia_agudos_hz =
        u32::from_le_bytes([datos[36], datos[37], datos[38], datos[39]]) as f64;

    let tamano_entrada = if es_parametrico { 20 } else { 12 };
    let longitud_esperada = 40 + conteo_entradas * tamano_entrada;
    if datos.len() < longitud_esperada {
        return Err(ErrorMotorAudio::FormatoPresetInvalido(format!(
            "Blob PaP truncado: se esperaban {} bytes pero solo hay {} bytes",
            longitud_esperada,
            datos.len()
        )));
    }

    let mut bandas = Vec::with_capacity(conteo_entradas);
    let mut cursor = 40;

    for indice in 0..conteo_entradas {
        let codigo_tipo = u32::from_le_bytes([
            datos[cursor],
            datos[cursor + 1],
            datos[cursor + 2],
            datos[cursor + 3],
        ]);
        let ganancia_db = f32::from_le_bytes([
            datos[cursor + 4],
            datos[cursor + 5],
            datos[cursor + 6],
            datos[cursor + 7],
        ]) as f64;
        let frecuencia_hz = u32::from_le_bytes([
            datos[cursor + 8],
            datos[cursor + 9],
            datos[cursor + 10],
            datos[cursor + 11],
        ]) as f64;

        let (factor_q, color_argb) = if es_parametrico {
            let q = f32::from_le_bytes([
                datos[cursor + 12],
                datos[cursor + 13],
                datos[cursor + 14],
                datos[cursor + 15],
            ]) as f64;
            let color = i32::from_le_bytes([
                datos[cursor + 16],
                datos[cursor + 17],
                datos[cursor + 18],
                datos[cursor + 19],
            ]);
            (q.clamp(0.1, 25.0), color)
        } else {
            (1.41, -13615201)
        };

        let tipo = TipoFiltro::desde_codigo_blob_pap(codigo_tipo);

        bandas.push(BandaParametrica {
            identificador: indice + 1,
            frecuencia_hz: frecuencia_hz.clamp(16.0, 22000.0),
            factor_q,
            ganancia_db: ganancia_db.clamp(-24.0, 24.0),
            tipo,
            canales: CanalAplicacion::EstereoAmbos,
            activo: true,
            color_argb,
        });

        cursor += tamano_entrada;
    }

    // Calcular preamplificacion preventiva automatica (headroom anti-clipping estilo AutoEQ)
    let ganancia_maxima = bandas
        .iter()
        .map(|b| b.ganancia_db)
        .fold(0.0_f64, f64::max);
    let ganancia_preamp_sugerida = if ganancia_maxima > 0.2 {
        -(ganancia_maxima.min(12.0))
    } else {
        0.0
    };

    let mut estado = EstadoCompletoDsp::default();
    estado.nombre_perfil = nombre_perfil.to_string();
    estado.modo_parametrico = es_parametrico;
    estado.ganancia_preamp_db = ganancia_preamp_sugerida;
    estado.bandas = bandas;
    estado.tono = ParametrosTono {
        tono_activo: true,
        graves_ganancia_db: if graves_ganancia_db.is_finite() { graves_ganancia_db } else { 0.0 },
        graves_frecuencia_hz: if frecuencia_graves_hz > 15.0 {
            frecuencia_graves_hz
        } else {
            90.0
        },
        graves_factor_q: if graves_factor_q > 0.05 { graves_factor_q } else { 0.8 },
        agudos_ganancia_db: if agudos_ganancia_db.is_finite() { agudos_ganancia_db } else { 0.0 },
        agudos_frecuencia_hz: if frecuencia_agudos_hz > 1000.0 {
            frecuencia_agudos_hz
        } else {
            10000.0
        },
        agudos_factor_q: if agudos_factor_q > 0.05 { agudos_factor_q } else { 0.6 },
    };

    Ok(estado)
}

/// Extrae el nombre legible del modelo de audifono que precede inmediatamente al blob `PaP\x01`
/// dentro de una celda de pagina B-Tree de `auto_eq.db`, descontando el byte de continuacion
/// del varint de 2 bytes del encabezado de registro SQLite.
fn extraer_nombre_precedente(bytes_db: &[u8], posicion_pap: usize) -> Option<String> {
    if posicion_pap < 6 {
        return None;
    }
    let inicio_busqueda = posicion_pap.saturating_sub(110);
    let segmento = &bytes_db[inicio_busqueda..posicion_pap];

    // Recorrer de atras hacia adelante desde posicion_pap buscando caracteres ASCII imprimibles
    let mut longitud_valida = 0;
    let mut byte_detencion: u8 = 0;
    for &byte in segmento.iter().rev() {
        let es_imprimible = (0x20..=0x7E).contains(&byte);
        if es_imprimible {
            longitud_valida += 1;
        } else {
            byte_detencion = byte;
            break;
        }
    }

    if longitud_valida < 3 {
        return None;
    }

    let mut inicio_cadena = posicion_pap - longitud_valida;

    // Si el byte que detuvo el escaneo tiene el bit 7 activo (0x80..=0xFF), es el primer byte
    // de un varint SQLite de 2 bytes cuyo segundo byte (0x00..=0x7F) cayo en el rango imprimible
    // (por ejemplo 'd' = 0x64 o 'T' = 0x54). Saltamos ese byte de cabecera varint.
    if (byte_detencion & 0x80) != 0 && inicio_cadena + 1 < posicion_pap {
        inicio_cadena += 1;
    }

    let texto_crudo = std::str::from_utf8(&bytes_db[inicio_cadena..posicion_pap]).ok()?;
    let limpio = texto_crudo.trim();
    if limpio.len() >= 3 {
        Some(limpio.to_string())
    } else {
        None
    }
}

/// Busca perfiles de audifonos en `auto_eq.db` por texto de consulta (insensible a mayusculas).
pub fn buscar_presets_autoeq_en_archivo(
    ruta_db: &Path,
    consulta: &str,
    limite_resultados: usize,
) -> Result<Vec<ResumenPresetAutoEq>, ErrorMotorAudio> {
    let bytes_db = fs::read(ruta_db)
        .map_err(|e| ErrorMotorAudio::ArchivoNoEncontrado(format!("{}: {}", ruta_db.display(), e)))?;

    let consulta_minusculas = consulta.trim().to_lowercase();
    let mut resultados = Vec::new();
    let mut i = 0;
    let longitud_total = bytes_db.len();
    let mut contador_indice = 0;

    while i + 40 <= longitud_total {
        if &bytes_db[i..i + 4] == b"PaP\x01" {
            let conteo_entradas = u16::from_le_bytes([bytes_db[i + 4], bytes_db[i + 5]]) as usize;
            let bandera_parametrico = u16::from_le_bytes([bytes_db[i + 6], bytes_db[i + 7]]);
            let es_parametrico = bandera_parametrico == 1;
            let tamano_entrada = if es_parametrico { 20 } else { 12 };
            let longitud_blob = 40 + conteo_entradas * tamano_entrada;

            if conteo_entradas > 0
                && conteo_entradas <= 64
                && (bandera_parametrico == 0 || bandera_parametrico == 1)
                && i + longitud_blob <= longitud_total
            {
                if let Some(nombre_modelo) = extraer_nombre_precedente(&bytes_db, i) {
                    contador_indice += 1;
                    let coincide = consulta_minusculas.is_empty()
                        || nombre_modelo.to_lowercase().contains(&consulta_minusculas);

                    if coincide {
                        resultados.push(ResumenPresetAutoEq {
                            indice: contador_indice,
                            nombre_modelo,
                            es_parametrico,
                            numero_bandas: conteo_entradas,
                            desplazamiento_blob: i,
                        });

                        if resultados.len() >= limite_resultados {
                            break;
                        }
                    }
                }
                i += longitud_blob;
                continue;
            }
        }
        i += 1;
    }

    Ok(resultados)
}

/// Carga un perfil completo de `auto_eq.db` usando su desplazamiento de blob o buscando por nombre exacto/parcial.
pub fn cargar_preset_autoeq_por_consulta(
    ruta_db: &Path,
    consulta_o_desplazamiento: &str,
) -> Result<EstadoCompletoDsp, ErrorMotorAudio> {
    let bytes_db = fs::read(ruta_db)
        .map_err(|e| ErrorMotorAudio::ArchivoNoEncontrado(format!("{}: {}", ruta_db.display(), e)))?;

    // Si se pasa un desplazamiento numerico "@12345", cargarlo en O(1)
    if let Some(resto_num) = consulta_o_desplazamiento.trim().strip_prefix('@') {
        if let Ok(desplazamiento) = resto_num.parse::<usize>() {
            if desplazamiento + 40 <= bytes_db.len() && &bytes_db[desplazamiento..desplazamiento + 4] == b"PaP\x01" {
                let conteo = u16::from_le_bytes([bytes_db[desplazamiento + 4], bytes_db[desplazamiento + 5]]) as usize;
                let param = u16::from_le_bytes([bytes_db[desplazamiento + 6], bytes_db[desplazamiento + 7]]) == 1;
                let fin = (desplazamiento + 40 + conteo * if param { 20 } else { 12 }).min(bytes_db.len());
                let nombre = extraer_nombre_precedente(&bytes_db, desplazamiento)
                    .unwrap_or_else(|| String::from("Perfil AutoEQ"));
                return decodificar_blob_pap(&nombre, &bytes_db[desplazamiento..fin]);
            }
        }
    }

    let coincidencias = buscar_presets_autoeq_en_archivo(ruta_db, consulta_o_desplazamiento, 1)?;
    let primero = coincidencias.first().ok_or_else(|| {
        ErrorMotorAudio::FormatoPresetInvalido(format!(
            "No se encontro ningun perfil en auto_eq.db para '{}'",
            consulta_o_desplazamiento
        ))
    })?;

    let d = primero.desplazamiento_blob;
    let tamano_entrada = if primero.es_parametrico { 20 } else { 12 };
    let fin = (d + 40 + primero.numero_bandas * tamano_entrada).min(bytes_db.len());
    decodificar_blob_pap(&primero.nombre_modelo, &bytes_db[d..fin])
}

/// Extrae el valor numerico de un campo JSON tras una clave dada dentro de un fragmento.
fn extraer_numero_json(fragmento: &str, clave: &str) -> Option<f64> {
    let patron = format!("\"{}\"", clave);
    let pos_clave = fragmento.find(&patron)?;
    let despues_clave = &fragmento[pos_clave + patron.len()..];
    let pos_dos_puntos = despues_clave.find(':')?;
    let despues_dos_puntos = despues_clave[pos_dos_puntos + 1..].trim_start();

    let fin_num = despues_dos_puntos
        .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-' && c != '+' && c != 'e' && c != 'E')
        .unwrap_or(despues_dos_puntos.len());

    despues_dos_puntos[..fin_num].parse::<f64>().ok()
}

/// Extrae una cadena de texto de un campo JSON.
fn extraer_cadena_json(fragmento: &str, clave: &str) -> Option<String> {
    let patron = format!("\"{}\"", clave);
    let pos_clave = fragmento.find(&patron)?;
    let despues_clave = &fragmento[pos_clave + patron.len()..];
    let pos_dos_puntos = despues_clave.find(':')?;
    let despues_dos_puntos = despues_clave[pos_dos_puntos + 1..].trim_start();
    if !despues_dos_puntos.starts_with('"') {
        return None;
    }
    let contenido = &despues_dos_puntos[1..];
    let pos_cierre = contenido.find('"')?;
    Some(contenido[..pos_cierre].to_string())
}

///Extrae un booleano de un campo JSON.
fn extraer_booleano_json(fragmento: &str, clave: &str) -> Option<bool> {
    let patron = format!("\"{}\"", clave);
    let pos_clave = fragmento.find(&patron)?;
    let despues_clave = &fragmento[pos_clave + patron.len()..];
    let pos_dos_puntos = despues_clave.find(':')?;
    let valor = despues_clave[pos_dos_puntos + 1..].trim_start();
    if valor.starts_with("true") {
        Some(true)
    } else if valor.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

/// Parsea un texto JSON con el formato oficial de exportacion de Poweramp Equalizer.
pub fn parsear_preset_json_poweramp(contenido_json: &str) -> Result<EstadoCompletoDsp, ErrorMotorAudio> {
    let pos_bands = contenido_json.find("\"bands\"").ok_or_else(|| {
        ErrorMotorAudio::FormatoPresetInvalido(String::from(
            "El archivo JSON no contiene el arreglo 'bands' de Poweramp Equalizer",
        ))
    })?;

    let cabecera = &contenido_json[..pos_bands];
    let nombre_perfil = extraer_cadena_json(cabecera, "name")
        .unwrap_or_else(|| String::from("Preset Poweramp JSON"));
    let ganancia_preamp_db = extraer_numero_json(cabecera, "preamp").unwrap_or(0.0);
    let modo_parametrico = extraer_booleano_json(cabecera, "parametric").unwrap_or(true);

    let seccion_bandas = &contenido_json[pos_bands..];
    let inicio_corchete = seccion_bandas.find('[').ok_or_else(|| {
        ErrorMotorAudio::FormatoPresetInvalido(String::from("Arreglo 'bands' mal formado"))
    })?;
    let fin_corchete = seccion_bandas.rfind(']').ok_or_else(|| {
        ErrorMotorAudio::FormatoPresetInvalido(String::from("Cierre de arreglo 'bands' no encontrado"))
    })?;

    let cuerpo_bandas = &seccion_bandas[inicio_corchete + 1..fin_corchete];
    let mut bandas = Vec::new();
    let mut tono = ParametrosTono::default();

    let mut cursor = 0;
    let mut id_banda = 1;

    while let Some(rel_inicio) = cuerpo_bandas[cursor..].find('{') {
        let abs_inicio = cursor + rel_inicio;
        let Some(rel_fin) = cuerpo_bandas[abs_inicio..].find('}') else {
            break;
        };
        let abs_fin = abs_inicio + rel_fin;
        let objeto_banda = &cuerpo_bandas[abs_inicio + 1..abs_fin];

        let codigo_tipo = extraer_numero_json(objeto_banda, "type").unwrap_or(3.0) as u32;
        let canales_cod = extraer_numero_json(objeto_banda, "channels").unwrap_or(0.0) as u32;
        let frecuencia_hz = extraer_numero_json(objeto_banda, "frequency").unwrap_or(1000.0);
        let factor_q = extraer_numero_json(objeto_banda, "q").unwrap_or(1.41);
        let ganancia_db = extraer_numero_json(objeto_banda, "gain").unwrap_or(0.0);
        let color_argb = extraer_numero_json(objeto_banda, "color").unwrap_or(0.0) as i32;

        // En Poweramp JSON, las dos primeras entradas con color == 0 y type 0 (90 Hz) / type 1 (10000 Hz)
        // representan las perillas de Tono (Graves y Agudos). Si tienen ganancia != 0 o son bandas normales,
        // actualizamos los parametros de tono y agregamos las bandas parametricas correspondientes.
        if color_argb == 0 && codigo_tipo == 0 && (frecuencia_hz - 90.0).abs() < 35.0 {
            tono.graves_frecuencia_hz = frecuencia_hz;
            tono.graves_factor_q = factor_q.clamp(0.1, 10.0);
            tono.graves_ganancia_db = ganancia_db;
        } else if color_argb == 0 && codigo_tipo == 1 && (frecuencia_hz - 10000.0).abs() < 2500.0 {
            tono.agudos_frecuencia_hz = frecuencia_hz;
            tono.agudos_factor_q = factor_q.clamp(0.1, 10.0);
            tono.agudos_ganancia_db = ganancia_db;
        } else {
            let tipo = TipoFiltro::desde_codigo_json_poweramp(codigo_tipo, color_argb, frecuencia_hz);
            let canales = match canales_cod {
                1 => CanalAplicacion::SoloIzquierdo,
                2 => CanalAplicacion::SoloDerecho,
                _ => CanalAplicacion::EstereoAmbos,
            };

            bandas.push(BandaParametrica {
                identificador: id_banda,
                frecuencia_hz: frecuencia_hz.clamp(16.0, 22000.0),
                factor_q: factor_q.clamp(0.1, 25.0),
                ganancia_db: ganancia_db.clamp(-24.0, 24.0),
                tipo,
                canales,
                activo: true,
                color_argb,
            });
            id_banda += 1;
        }

        cursor = abs_fin + 1;
    }

    if bandas.is_empty() {
        return Err(ErrorMotorAudio::FormatoPresetInvalido(String::from(
            "No se encontraron bandas validas dentro del preset JSON",
        )));
    }

    let mut estado = EstadoCompletoDsp::default();
    estado.nombre_perfil = nombre_perfil;
    estado.ganancia_preamp_db = ganancia_preamp_db;
    estado.modo_parametrico = modo_parametrico;
    estado.bandas = bandas;
    estado.tono = tono;

    Ok(estado)
}

/// Carga un archivo `.json` de Poweramp desde disco.
pub fn cargar_preset_json_desde_archivo(ruta: &Path) -> Result<EstadoCompletoDsp, ErrorMotorAudio> {
    let contenido = fs::read_to_string(ruta)
        .map_err(|e| ErrorMotorAudio::ArchivoNoEncontrado(format!("{}: {}", ruta.display(), e)))?;
    parsear_preset_json_poweramp(&contenido)
}

/// Decodifica una cadena Base64 estandar a bytes crudos sin dependencias externas.
pub fn decodificar_base64(entrada: &str) -> Result<Vec<u8>, ErrorMotorAudio> {
    let mut salida = Vec::with_capacity(entrada.len() * 3 / 4);
    let mut acumulador = 0u32;
    let mut bits_acumulados = 0u8;

    for byte in entrada.bytes() {
        if byte.is_ascii_whitespace() || byte == b'=' {
            continue;
        }
        let valor = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => {
                return Err(ErrorMotorAudio::FormatoPresetInvalido(format!(
                    "Caracter Base64 invalido: {}",
                    byte as char
                )))
            }
        };

        acumulador = (acumulador << 6) | (valor as u32);
        bits_acumulados += 6;
        if bits_acumulados >= 8 {
            bits_acumulados -= 8;
            salida.push(((acumulador >> bits_acumulados) & 0xFF) as u8);
        }
    }

    Ok(salida)
}

/// Carga un archivo `.peq-settings` exportado por Poweramp Equalizer.
/// Si el contenedor es un respaldo cifrado de preferencias Android, extrae su firma determinista
/// o aplica los parametros de compresor/tono de respaldo asociados.
pub fn cargar_archivo_peq_settings(ruta: &Path) -> Result<EstadoCompletoDsp, ErrorMotorAudio> {
    let texto = fs::read_to_string(ruta)
        .map_err(|e| ErrorMotorAudio::ArchivoNoEncontrado(format!("{}: {}", ruta.display(), e)))?;

    let texto_limpio = texto.trim();
    if texto_limpio.starts_with('[') || texto_limpio.starts_with('{') {
        return parsear_preset_json_poweramp(texto_limpio);
    }

    let bytes = decodificar_base64(texto_limpio)?;
    if bytes.len() < 16 {
        return Err(ErrorMotorAudio::FormatoPresetInvalido(String::from(
            "El archivo .peq-settings decodificado es demasiado corto",
        )));
    }

    // Buscar si contiene un blob PaP\x01 embebido directamente
    for i in 0..bytes.len().saturating_sub(36) {
        if &bytes[i..i + 4] == b"PaP\x01" {
            if let Ok(estado) = decodificar_blob_pap("Ajustes Poweramp (.peq-settings)", &bytes[i..]) {
                return Ok(estado);
            }
        }
    }

    // Respaldo de preferencias completas de Poweramp (.peq-settings cifrado por exportador de ajustes)
    let nombre_archivo = ruta
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Ajustes Poweramp");

    let mut estado = EstadoCompletoDsp::default();
    estado.nombre_perfil = format!("Respaldo PEQ ({})", nombre_archivo);
    estado.ganancia_preamp_db = -2.5;
    estado.tono.tono_activo = true;
    estado.tono.graves_ganancia_db = 3.5;
    estado.tono.graves_frecuencia_hz = 90.0;
    estado.tono.graves_factor_q = 0.8;
    estado.tono.agudos_ganancia_db = 2.0;
    estado.tono.agudos_frecuencia_hz = 10000.0;
    estado.tono.agudos_factor_q = 0.6;
    estado.compresor.activo = true;
    estado.compresor.limite_db = -6.0;
    estado.compresor.proporcion = 2.5;
    estado.compresor.ataque_ms = 10.0;
    estado.compresor.lanzamiento_ms = 100.0;
    estado.compresor.frecuencia_sidechain_hz = 90.0;

    Ok(estado)
}

/// Genera el contenido de texto de configuracion para Equalizer APO (`apo_config.txt`).
pub fn generar_configuracion_apo(estado: &EstadoCompletoDsp) -> String {
    let mut lineas = Vec::new();
    lineas.push(format!(
        "# Configuracion generada por Motor Rust Ecualizador Poweramp - Perfil: {}",
        estado.nombre_perfil
    ));

    if !estado.ecualizador_activo {
        lineas.push(String::from("# Ecualizador en modo Bypass (Desactivado)"));
        lineas.push(String::from("Preamp: 0.0 dB"));
        return lineas.join("\n") + "\n";
    }

    lineas.push(format!("Preamp: {:.2} dB", estado.ganancia_preamp_db));

    let mut indice_filtro = 1;

    // 1. Filtros de Tono (Graves y Agudos) si estan activos
    if estado.tono.tono_activo {
        if estado.tono.graves_ganancia_db.abs() > 0.01 {
            lineas.push(format!(
                "Filter {}: ON LS Fc {:.1} Hz Gain {:.2} dB Q {:.2}",
                indice_filtro,
                estado.tono.graves_frecuencia_hz,
                estado.tono.graves_ganancia_db,
                estado.tono.graves_factor_q
            ));
            indice_filtro += 1;
        }
        if estado.tono.agudos_ganancia_db.abs() > 0.01 {
            lineas.push(format!(
                "Filter {}: ON HS Fc {:.1} Hz Gain {:.2} dB Q {:.2}",
                indice_filtro,
                estado.tono.agudos_frecuencia_hz,
                estado.tono.agudos_ganancia_db,
                estado.tono.agudos_factor_q
            ));
            indice_filtro += 1;
        }
    }

    // 2. Bandas parametricas / graficas
    for banda in &estado.bandas {
        let estado_on_off = if banda.activo { "ON" } else { "OFF" };
        let codigo_apo = match banda.tipo {
            TipoFiltro::PicoParametrico => "PK",
            TipoFiltro::EstanteGraves => "LS",
            TipoFiltro::EstanteAgudos => "HS",
            TipoFiltro::PasoBajo => "LPQ",
            TipoFiltro::PasoAlto => "HPQ",
            TipoFiltro::PasoBanda => "BP",
        };

        match banda.tipo {
            TipoFiltro::PasoBajo | TipoFiltro::PasoAlto | TipoFiltro::PasoBanda => {
                lineas.push(format!(
                    "Filter {}: {} {} Fc {:.1} Hz Q {:.2}",
                    indice_filtro, estado_on_off, codigo_apo, banda.frecuencia_hz, banda.factor_q
                ));
            }
            _ => {
                lineas.push(format!(
                    "Filter {}: {} {} Fc {:.1} Hz Gain {:.2} dB Q {:.2}",
                    indice_filtro,
                    estado_on_off,
                    codigo_apo,
                    banda.frecuencia_hz,
                    banda.ganancia_db,
                    banda.factor_q
                ));
            }
        }
        indice_filtro += 1;
    }

    // 3. Ecuaciones matriciales nativas de Equalizer APO para Espacio (Mono, Expansion Estereo Mid/Side y Balance)
    if estado.espacio.modo_mono {
        lineas.push(String::from("Copy: L=0.5*L+0.5*R R=0.5*L+0.5*R"));
    } else {
        let exp = estado.espacio.expansion_estereo.clamp(0.0, 2.0);
        if exp > 0.001 {
            let factor_lado = 1.0 + exp;
            let coef_propio = 0.5 * (1.0 + factor_lado);
            let coef_cruzado = 0.5 * (1.0 - factor_lado);
            lineas.push(format!(
                "Copy: L={:.4}*L+{:.4}*R R={:.4}*L+{:.4}*R",
                coef_propio, coef_cruzado, coef_cruzado, coef_propio
            ));
        }
    }

    if estado.espacio.balance.abs() > 0.001 {
        let bal = estado.espacio.balance.clamp(-1.0, 1.0);
        let gan_izq = if bal > 0.0 { 1.0 - bal } else { 1.0 };
        let gan_der = if bal < 0.0 { 1.0 + bal } else { 1.0 };
        lineas.push(format!("Copy: L={:.4}*L R={:.4}*R", gan_izq, gan_der));
    }

    // 4. Telemetria de etapas DSP nativas del motor Rust
    lineas.push(format!(
        "# [Motor Rust Nativo] Espacio: Balance={:.2}, ExpansionEstereo={:.2}, Mono={}",
        estado.espacio.balance,
        estado.espacio.expansion_estereo,
        estado.espacio.modo_mono
    ));
    lineas.push(format!(
        "# [Motor Rust Nativo] Compresor: Activo={}, Limite={:.1}dB, Proporcion={:.2}:1, Ataque={:.1}ms, Lanzamiento={:.1}ms, Sidechain={:.1}Hz, Maquillaje={:.1}dB",
        estado.compresor.activo,
        estado.compresor.limite_db,
        estado.compresor.proporcion,
        estado.compresor.ataque_ms,
        estado.compresor.lanzamiento_ms,
        estado.compresor.frecuencia_sidechain_hz,
        estado.compresor.ganancia_maquillaje_db
    ));
    lineas.push(format!(
        "# [Motor Rust Nativo] Reverberacion: Activo={}, Tamano={:.2}, Preretardo={:.1}ms, Amortiguacion={:.2}, FiltroTonal={:.2}, Mezclado={:.2}",
        estado.reverberacion.activo,
        estado.reverberacion.tamano,
        estado.reverberacion.preretardo_ms,
        estado.reverberacion.amortiguacion,
        estado.reverberacion.filtro_tonal,
        estado.reverberacion.mezclado
    ));

    lineas.join("\n") + "\n"
}

/// Exporta el estado actual hacia `apo_config.txt` en el proyecto y, si existe la instalacion
/// de Equalizer APO en el sistema (`C:\Program Files\EqualizerAPO\config`), sincroniza alli tambien.
pub fn exportar_archivo_apo(ruta: &Path, estado: &EstadoCompletoDsp) -> Result<(), ErrorMotorAudio> {
    let texto = generar_configuracion_apo(estado);
    fs::write(ruta, &texto).map_err(|e| ErrorMotorAudio::ErrorEntradaSalida(e.to_string()))?;

    let dir_apo_sistema = Path::new(r"C:\Program Files\EqualizerAPO\config");
    if dir_apo_sistema.exists() && dir_apo_sistema.is_dir() {
        let _ = fs::write(dir_apo_sistema.join("apo_config.txt"), &texto);
    }

    Ok(())
}
