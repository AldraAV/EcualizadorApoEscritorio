//! Decodificador Nativo en Rust del Formato Binario Android XML (`AXML`) extraido por REA
//! desde `Equalizador Poweramp.apk`.
//! Permite reconstruir la jerarquia exacta de vistas, controles, contenedores y skins
//! de Poweramp Equalizer (`activity_peq_main.xml`, `merge_eq_equalizer_tab_common.xml`,
//! `merge_eq_vol_tab.xml`, `merge_eq_reverb_tab.xml`, `merge_navbar.xml`, `peq_builtin_skins.xml`).

use std::fs;
use std::path::Path;

const RES_STRING_POOL_TYPE: u16 = 0x0001;
const RES_XML_START_ELEMENT_TYPE: u16 = 0x0102;
const RES_XML_END_ELEMENT_TYPE: u16 = 0x0103;
const UTF8_FLAG: u32 = 1 << 8;

/// Decodifica la tabla de cadenas (`ResStringPool`) de un archivo binario AXML de Android.
fn decodificar_tabla_cadenas_axml(bytes: &[u8], offset_chunk: usize) -> Vec<String> {
    if offset_chunk + 28 > bytes.len() {
        return Vec::new();
    }
    let conteo_cadenas =
        u32::from_le_bytes(bytes[offset_chunk + 8..offset_chunk + 12].try_into().unwrap()) as usize;
    let banderas =
        u32::from_le_bytes(bytes[offset_chunk + 16..offset_chunk + 20].try_into().unwrap());
    let inicio_cadenas =
        u32::from_le_bytes(bytes[offset_chunk + 20..offset_chunk + 24].try_into().unwrap()) as usize;
    let es_utf8 = (banderas & UTF8_FLAG) != 0;

    let base_offsets = offset_chunk + 28;
    let base_datos = offset_chunk + inicio_cadenas;
    let mut resultado = Vec::with_capacity(conteo_cadenas);

    for i in 0..conteo_cadenas {
        let pos_idx = base_offsets + i * 4;
        if pos_idx + 4 > bytes.len() {
            break;
        }
        let desp = u32::from_le_bytes(bytes[pos_idx..pos_idx + 4].try_into().unwrap()) as usize;
        let mut cursor = base_datos + desp;
        if cursor >= bytes.len() {
            resultado.push(String::new());
            continue;
        }

        if es_utf8 {
            if (bytes[cursor] & 0x80) != 0 {
                cursor += 2;
            } else {
                cursor += 1;
            }
            if cursor >= bytes.len() {
                resultado.push(String::new());
                continue;
            }
            let lon_utf8 = if (bytes[cursor] & 0x80) != 0 {
                let b1 = (bytes[cursor] & 0x7F) as usize;
                let b2 = *bytes.get(cursor + 1).unwrap_or(&0) as usize;
                cursor += 2;
                (b1 << 8) | b2
            } else {
                let b1 = bytes[cursor] as usize;
                cursor += 1;
                b1
            };
            let fin = (cursor + lon_utf8).min(bytes.len());
            resultado.push(String::from_utf8_lossy(&bytes[cursor..fin]).into_owned());
        } else {
            if cursor + 2 > bytes.len() {
                resultado.push(String::new());
                continue;
            }
            let mut lon_u16 = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]) as usize;
            cursor += 2;
            if (lon_u16 & 0x8000) != 0 && cursor + 2 <= bytes.len() {
                let bajo = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]) as usize;
                lon_u16 = ((lon_u16 & 0x7FFF) << 16) | bajo;
                cursor += 2;
            }
            let mut unidades = Vec::with_capacity(lon_u16);
            for _ in 0..lon_u16 {
                if cursor + 2 > bytes.len() {
                    break;
                }
                unidades.push(u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]));
                cursor += 2;
            }
            resultado.push(String::from_utf16_lossy(&unidades));
        }
    }

    resultado
}

/// Decodifica un archivo binario AXML de Android y devuelve su estructura XML legible.
pub fn decodificar_archivo_axml(ruta: &Path) -> Result<String, String> {
    let bytes = fs::read(ruta).map_err(|e| format!("Error al leer {}: {}", ruta.display(), e))?;
    if bytes.len() < 8 {
        return Err(String::from("Archivo AXML demasiado corto"));
    }

    let mut cadenas: Vec<String> = Vec::new();
    let mut salida = String::new();
    let mut nivel_indentacion = 0usize;
    let mut cursor = 8usize;

    while cursor + 8 <= bytes.len() {
        let tipo_chunk = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]);
        let tamano_cabecera = u16::from_le_bytes([bytes[cursor + 2], bytes[cursor + 3]]) as usize;
        let tamano_chunk =
            u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap()) as usize;
        if tamano_chunk < 8 || cursor + tamano_chunk > bytes.len() {
            break;
        }

        match tipo_chunk {
            RES_STRING_POOL_TYPE => {
                cadenas = decodificar_tabla_cadenas_axml(&bytes, cursor);
            }
            RES_XML_START_ELEMENT_TYPE => {
                if cursor + 36 <= bytes.len() {
                    let idx_nombre =
                        i32::from_le_bytes(bytes[cursor + 20..cursor + 24].try_into().unwrap());
                    let tam_atrib =
                        u16::from_le_bytes([bytes[cursor + 26], bytes[cursor + 27]]) as usize;
                    let num_atrib =
                        u16::from_le_bytes([bytes[cursor + 28], bytes[cursor + 29]]) as usize;

                    let nombre_etiqueta =
                        if idx_nombre >= 0 && (idx_nombre as usize) < cadenas.len() {
                            &cadenas[idx_nombre as usize]
                        } else {
                            "Nodo"
                        };

                    let sangria = "  ".repeat(nivel_indentacion);
                    salida.push_str(&format!("{}<{}", sangria, nombre_etiqueta));

                    let mut pos_atrib = cursor + tamano_cabecera.max(36);
                    for _ in 0..num_atrib {
                        if pos_atrib + 20 > cursor + tamano_chunk {
                            break;
                        }
                        let idx_nom_atrib =
                            i32::from_le_bytes(bytes[pos_atrib + 4..pos_atrib + 8].try_into().unwrap());
                        let idx_val_crudo =
                            i32::from_le_bytes(bytes[pos_atrib + 8..pos_atrib + 12].try_into().unwrap());
                        let tipo_dato = bytes[pos_atrib + 15];
                        let dato_num =
                            u32::from_le_bytes(bytes[pos_atrib + 16..pos_atrib + 20].try_into().unwrap());

                        let nom_atrib =
                            if idx_nom_atrib >= 0 && (idx_nom_atrib as usize) < cadenas.len() {
                                &cadenas[idx_nom_atrib as usize]
                            } else {
                                "attr"
                            };

                        let val_texto =
                            if idx_val_crudo >= 0 && (idx_val_crudo as usize) < cadenas.len() {
                                cadenas[idx_val_crudo as usize].clone()
                            } else {
                                match tipo_dato {
                                    0x01 => format!("@0x{:08X}", dato_num),
                                    0x02 => format!("?0x{:08X}", dato_num),
                                    0x04 => format!("{:.2}", f32::from_bits(dato_num)),
                                    0x10 | 0x11 => format!("0x{:X}", dato_num),
                                    0x12 => {
                                        if dato_num != 0 {
                                            String::from("true")
                                        } else {
                                            String::from("false")
                                        }
                                    }
                                    0x1C..=0x1F => format!("#{:08X}", dato_num),
                                    _ => format!("0x{:X}", dato_num),
                                }
                            };

                        if !nom_atrib.is_empty() {
                            salida.push_str(&format!(" {}=\"{}\"", nom_atrib, val_texto));
                        }
                        pos_atrib += tam_atrib.max(20);
                    }

                    salida.push_str(">\n");
                    nivel_indentacion += 1;
                }
            }
            RES_XML_END_ELEMENT_TYPE => {
                if cursor + 24 <= bytes.len() {
                    nivel_indentacion = nivel_indentacion.saturating_sub(1);
                    let idx_nombre =
                        i32::from_le_bytes(bytes[cursor + 20..cursor + 24].try_into().unwrap());
                    let nombre_etiqueta =
                        if idx_nombre >= 0 && (idx_nombre as usize) < cadenas.len() {
                            &cadenas[idx_nombre as usize]
                        } else {
                            "Nodo"
                        };
                    let sangria = "  ".repeat(nivel_indentacion);
                    salida.push_str(&format!("{}</{}>\n", sangria, nombre_etiqueta));
                }
            }
            _ => {}
        }

        cursor += tamano_chunk;
    }

    Ok(salida)
}
