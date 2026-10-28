export type TipoFiltro =
  | 'PasoAlto'
  | 'PasoBajo'
  | 'PasoBanda'
  | 'PicoParametrico'
  | 'EstanteGraves'
  | 'EstanteAgudos';

export type VistaActivaPoweramp =
  | 'ecualizador'
  | 'espacio'
  | 'compresor'
  | 'reverberacion';

export interface BandaAudio {
  identificador: number;
  frecuenciaHz: number;
  factorQ: number;
  gananciaDb: number;
  tipo: TipoFiltro;
  activo: boolean;
  etiqueta: string;
  nombreHumano?: string;
  descripcion?: string;
}

export interface ParametrosTonoUi {
  tonoActivo: boolean;
  gravesGananciaDb: number;
  gravesFrecuenciaHz: number;
  gravesFactorQ: number;
  agudosGananciaDb: number;
  agudosFrecuenciaHz: number;
  agudosFactorQ: number;
}

export interface ParametrosEspacioUi {
  balance: number;
  expansionEstereo: number;
  modoMono: boolean;
  tempo: number;
}

export interface ParametrosCompresorUi {
  activo: boolean;
  limiteDb: number;
  proporcion: number;
  ataqueMs: number;
  lanzamientoMs: number;
  frecuenciaSidechainHz: number;
  gananciaMaquillajeDb: number;
}

export interface ParametrosReverberacionUi {
  activo: boolean;
  tamano: number;
  preretardoMs: number;
  amortiguacion: number;
  filtroTonal: number;
  mezclado: number;
}

export interface PuntoCurvaRespuesta {
  frecuenciaHz: number;
  gananciaDb: number;
}

export interface ResumenPresetAutoEqUi {
  indice: number;
  nombreModelo: string;
  esParametrico: boolean;
  numeroBandas: number;
  desplazamientoBlob: number;
}

export interface EstadoEcualizador {
  frecuenciaMuestreo: number;
  gananciaPreampDb: number;
  limitadorActivo: boolean;
  compresorUmbralDb: number;
  compresorRatio: number;
  perfilActual: string;
  modoParametrico: boolean;
  bandas: BandaAudio[];
  tono: ParametrosTonoUi;
  espacio: ParametrosEspacioUi;
  compresor: ParametrosCompresorUi;
  reverberacion: ParametrosReverberacionUi;
}
