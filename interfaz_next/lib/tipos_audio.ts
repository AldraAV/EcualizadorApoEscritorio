// Tipos de datos para el Ecualizador Poweramp Soberano en Next.js / Tauri.
// Cumple al 100% con la Soberania Linguistica del Espanol y Cero Emojis.

export type TipoFiltro =
  | 'PasoAlto'
  | 'PasoBajo'
  | 'PasoBanda'
  | 'PicoParametrico'
  | 'EstanteGraves'
  | 'EstanteAgudos';

export type CanalAplicacion = 'EstereoAmbos' | 'SoloIzquierdo' | 'SoloDerecho';

export type VistaActivaPoweramp =
  | 'ecualizador'
  | 'espacio_compresor'
  | 'reverberacion'
  | 'autoeq_dispositivos';

export interface BandaParametricaRust {
  identificador: number;
  frecuencia_hz: number;
  factor_q: number;
  ganancia_db: number;
  tipo: TipoFiltro;
  canales: CanalAplicacion;
  activo: boolean;
  color_argb: number;
}

export interface BandaAudioUi {
  identificador: number;
  frecuenciaHz: number;
  factorQ: number;
  gananciaDb: number;
  tipo: TipoFiltro;
  activo: boolean;
  etiqueta: string;
  nombreHumano: string;
  descripcion: string;
}

export interface ParametrosTonoRust {
  tono_activo: boolean;
  graves_ganancia_db: number;
  graves_frecuencia_hz: number;
  graves_factor_q: number;
  agudos_ganancia_db: number;
  agudos_frecuencia_hz: number;
  agudos_factor_q: number;
}

export interface ParametrosEspacioRust {
  balance: number;
  expansion_estereo: number;
  modo_mono: boolean;
}

export interface ParametrosCompresorRust {
  activo: boolean;
  limite_db: number;
  proporcion: number;
  ataque_ms: number;
  lanzamiento_ms: number;
  frecuencia_sidechain_hz: number;
  ganancia_maquillaje_db: number;
}

export interface ParametrosReverberacionRust {
  activo: boolean;
  tamano: number;
  preretardo_ms: number;
  amortiguacion: number;
  filtro_tonal: number;
  mezclado: number;
}

export interface EstadoCompletoDspRust {
  nombre_perfil: string;
  ecualizador_activo: boolean;
  modo_parametrico: boolean;
  frecuencia_muestreo_hz: number;
  ganancia_preamp_db: number;
  limitador_activo: boolean;
  bandas: BandaParametricaRust[];
  tono: ParametrosTonoRust;
  espacio: ParametrosEspacioRust;
  compresor: ParametrosCompresorRust;
  reverberacion: ParametrosReverberacionRust;
}

export interface TelemetriaUi {
  barras: number[];
  reduccion_compresor_db: number;
  pico_salida_db: number;
  puntos_curva: number[];
}

export interface ResumenPresetAutoEq {
  indice: number;
  nombre_modelo: string;
  es_parametrico: boolean;
  numero_bandas: number;
  desplazamiento_blob: number;
}

export interface DispositivoAudioWindows {
  identificador: string;
  nombre_amigable: string;
  es_predeterminado: boolean;
}
