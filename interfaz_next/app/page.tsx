'use client';

import React, { useEffect, useState, useCallback, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Sliders,
  Gauge,
  Waves,
  Headphones,
  Power,
  RotateCcw,
  Search,
  Cpu,
  Volume2,
  ShieldCheck,
  CheckCircle2,
  Download,
  Speaker,
} from 'lucide-react';

import { TarjetaBento } from '@/components/TarjetaBento';
import { FaderEsqueumorfico } from '@/components/FaderEsqueumorfico';
import { PerillaRotativa3D } from '@/components/PerillaRotativa3D';
import { VisualizadorEspectroAmbar } from '@/components/VisualizadorEspectroAmbar';
import {
  BandaAudioUi,
  BandaParametricaRust,
  DispositivoAudioWindows,
  EstadoCompletoDspRust,
  ResumenPresetAutoEq,
  TelemetriaUi,
  VistaActivaPoweramp,
} from '@/lib/tipos_audio';

// Adaptador seguro para llamadas IPC de Tauri v2
async function invocarTauri<T>(
  comando: string,
  argumentos?: Record<string, unknown>
): Promise<T | null> {
  if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<T>(comando, argumentos);
    } catch (error) {
      console.warn(`Error al invocar comando Tauri [${comando}]:`, error);
      return null;
    }
  }
  return null;
}

const BANDAS_INICIALES: BandaAudioUi[] = [
  {
    identificador: 1,
    frecuenciaHz: 31.5,
    factorQ: 1.41,
    gananciaDb: 4.0,
    tipo: 'EstanteGraves',
    activo: true,
    etiqueta: '31.5 Hz',
    nombreHumano: 'Sub-graves',
    descripcion: 'Vibracion profunda y sub-bajos',
  },
  {
    identificador: 2,
    frecuenciaHz: 63,
    factorQ: 1.41,
    gananciaDb: 3.2,
    tipo: 'PicoParametrico',
    activo: true,
    etiqueta: '63 Hz',
    nombreHumano: 'Graves',
    descripcion: 'Fuerza y empuje principal del bajo y el bombo',
  },
  {
    identificador: 3,
    frecuenciaHz: 125,
    factorQ: 1.41,
    gananciaDb: 1.5,
    tipo: 'PicoParametrico',
    activo: true,
    etiqueta: '125 Hz',
    nombreHumano: 'Cuerpo',
    descripcion: 'Golpe y plenitud de guitarras graves',
  },
  {
    identificador: 4,
    frecuenciaHz: 250,
    factorQ: 1.41,
    gananciaDb: 0.0,
    tipo: 'PicoParametrico',
    activo: true,
    etiqueta: '250 Hz',
    nombreHumano: 'Calidez',
    descripcion: 'Tono calido de voces e instrumentos',
  },
  {
    identificador: 5,
    frecuenciaHz: 500,
    factorQ: 1.41,
    gananciaDb: -1.0,
    tipo: 'PicoParametrico',
    activo: true,
    etiqueta: '500 Hz',
    nombreHumano: 'Voz Media',
    descripcion: 'Cuerpo de voces e instrumentos de viento',
  },
  {
    identificador: 6,
    frecuenciaHz: 1000,
    factorQ: 1.41,
    gananciaDb: -0.5,
    tipo: 'PicoParametrico',
    activo: true,
    etiqueta: '1 kHz',
    nombreHumano: 'Presencia',
    descripcion: 'Centro vocal, guitarras y ataque principal',
  },
  {
    identificador: 7,
    frecuenciaHz: 2000,
    factorQ: 1.41,
    gananciaDb: 1.0,
    tipo: 'PicoParametrico',
    activo: true,
    etiqueta: '2 kHz',
    nombreHumano: 'Claridad',
    descripcion: 'Definicion vocal e inteligibilidad',
  },
  {
    identificador: 8,
    frecuenciaHz: 4000,
    factorQ: 1.41,
    gananciaDb: 2.5,
    tipo: 'PicoParametrico',
    activo: true,
    etiqueta: '4 kHz',
    nombreHumano: 'Nitidez',
    descripcion: 'Ataque de bateria y detalles de cuerdas',
  },
  {
    identificador: 9,
    frecuenciaHz: 8000,
    factorQ: 1.41,
    gananciaDb: 4.0,
    tipo: 'PicoParametrico',
    activo: true,
    etiqueta: '8 kHz',
    nombreHumano: 'Brillo',
    descripcion: 'Platillos metalicos y definicion armonica',
  },
  {
    identificador: 10,
    frecuenciaHz: 16000,
    factorQ: 1.41,
    gananciaDb: 5.0,
    tipo: 'EstanteAgudos',
    activo: true,
    etiqueta: '16 kHz',
    nombreHumano: 'Aire Hi-Fi',
    descripcion: 'Sensacion de espacio abierto y fidelidad extrema',
  },
];

function formatearEtiquetaFrecuencia(frecuenciaHz: number): string {
  if (frecuenciaHz >= 1000) {
    return `${Math.round((frecuenciaHz / 1000) * 10) / 10} kHz`;
  }
  return `${Math.round(frecuenciaHz)} Hz`;
}

export default function PaginaPrincipalEcualizador() {
  const [vistaActiva, setVistaActiva] = useState<VistaActivaPoweramp>('ecualizador');
  const [bandas, setBandas] = useState<BandaAudioUi[]>(BANDAS_INICIALES);
  const [ecualizadorActivo, setEcualizadorActivo] = useState<boolean>(true);
  const [limitadorActivo, setLimitadorActivo] = useState<boolean>(true);
  const [gananciaPreampDb, setGananciaPreampDb] = useState<number>(0.0);
  const [nombrePerfil, setNombrePerfil] = useState<string>('Potencia Analogica (Poweramp)');
  const [frecuenciaMuestreo, setFrecuenciaMuestreo] = useState<number>(48000);

  // Controles de Tono
  const [tonoActivo, setTonoActivo] = useState<boolean>(true);
  const [gravesDb, setGravesDb] = useState<number>(0.0);
  const [agudosDb, setAgudosDb] = useState<number>(0.0);

  // Controles de Espacio (sin tempo)
  const [balance, setBalance] = useState<number>(0.0);
  const [expansionEstereo, setExpansionEstereo] = useState<number>(0.0);
  const [modoMono, setModoMono] = useState<boolean>(false);

  // Controles de Compresor (sin tempo)
  const [compresorActivo, setCompresorActivo] = useState<boolean>(true);
  const [compresorLimiteDb, setCompresorLimiteDb] = useState<number>(-6.0);
  const [compresorProporcion, setCompresorProporcion] = useState<number>(2.5);
  const [compresorAtaqueMs, setCompresorAtaqueMs] = useState<number>(10.0);
  const [compresorLanzamientoMs, setCompresorLanzamientoMs] = useState<number>(100.0);
  const [compresorSidechainHz, setCompresorSidechainHz] = useState<number>(90.0);
  const [compresorMaquillajeDb, setCompresorMaquillajeDb] = useState<number>(0.0);

  // Controles de Reverberacion
  const [reverbActivo, setReverbActivo] = useState<boolean>(false);
  const [reverbTamano, setReverbTamano] = useState<number>(0.5);
  const [reverbPreretardoMs, setReverbPreretardoMs] = useState<number>(15.0);
  const [reverbAmortiguacion, setReverbAmortiguacion] = useState<number>(0.5);
  const [reverbFiltroTonal, setReverbFiltroTonal] = useState<number>(0.5);
  const [reverbMezclado, setReverbMezclado] = useState<number>(0.2);

  // Telemetria en tiempo real
  const [barrasEspectro, setBarrasEspectro] = useState<number[]>([]);
  const [puntosCurva, setPuntosCurva] = useState<number[]>([]);
  const [reduccionCompresorDb, setReduccionCompresorDb] = useState<number>(0.0);
  const [picoSalidaDb, setPicoSalidaDb] = useState<number>(-18.0);
  const [motorRustConectado, setMotorRustConectado] = useState<boolean>(false);

  // AutoEQ y Dispositivos
  const [consultaAutoEq, setConsultaAutoEq] = useState<string>('JBL');
  const [resultadosAutoEq, setResultadosAutoEq] = useState<ResumenPresetAutoEq[]>([]);
  const [cargandoAutoEq, setCargandoAutoEq] = useState<boolean>(false);
  const [dispositivosAudio, setDispositivosAudio] = useState<DispositivoAudioWindows[]>([]);
  const [dispositivoSeleccionadoId, setDispositivoSeleccionadoId] = useState<string>('');
  const [mensajeEstado, setMensajeEstado] = useState<string>(
    'Motor DSP Soberano en Rust listo (64 bits, Tauri v2)'
  );

  // Construir objeto de estado completo para Rust
  const construirEstadoDspRust = useCallback((): EstadoCompletoDspRust => {
    const bandasRust: BandaParametricaRust[] = bandas.map((b) => ({
      identificador: b.identificador,
      frecuencia_hz: b.frecuenciaHz,
      factor_q: b.factorQ,
      ganancia_db: b.gananciaDb,
      tipo: b.tipo,
      canales: 'EstereoAmbos',
      activo: b.activo,
      color_argb: -6543440,
    }));

    return {
      nombre_perfil: nombrePerfil,
      ecualizador_activo: ecualizadorActivo,
      modo_parametrico: true,
      frecuencia_muestreo_hz: frecuenciaMuestreo,
      ganancia_preamp_db: gananciaPreampDb,
      limitador_activo: limitadorActivo,
      bandas: bandasRust,
      tono: {
        tono_activo: tonoActivo,
        graves_ganancia_db: gravesDb,
        graves_frecuencia_hz: 90.0,
        graves_factor_q: 0.8,
        agudos_ganancia_db: agudosDb,
        agudos_frecuencia_hz: 10000.0,
        agudos_factor_q: 0.6,
      },
      espacio: {
        balance,
        expansion_estereo: expansionEstereo,
        modo_mono: modoMono,
      },
      compresor: {
        activo: compresorActivo,
        limite_db: compresorLimiteDb,
        proporcion: compresorProporcion,
        ataque_ms: compresorAtaqueMs,
        lanzamiento_ms: compresorLanzamientoMs,
        frecuencia_sidechain_hz: compresorSidechainHz,
        ganancia_maquillaje_db: compresorMaquillajeDb,
      },
      reverberacion: {
        activo: reverbActivo,
        tamano: reverbTamano,
        preretardo_ms: reverbPreretardoMs,
        amortiguacion: reverbAmortiguacion,
        filtro_tonal: reverbFiltroTonal,
        mezclado: reverbMezclado,
      },
    };
  }, [
    bandas,
    nombrePerfil,
    ecualizadorActivo,
    frecuenciaMuestreo,
    gananciaPreampDb,
    limitadorActivo,
    tonoActivo,
    gravesDb,
    agudosDb,
    balance,
    expansionEstereo,
    modoMono,
    compresorActivo,
    compresorLimiteDb,
    compresorProporcion,
    compresorAtaqueMs,
    compresorLanzamientoMs,
    compresorSidechainHz,
    compresorMaquillajeDb,
    reverbActivo,
    reverbTamano,
    reverbPreretardoMs,
    reverbAmortiguacion,
    reverbFiltroTonal,
    reverbMezclado,
  ]);

  // Sincronizar hacia Rust cada vez que cambian parametros de audio
  const sincronizarHaciaRust = useCallback(async () => {
    const estado = construirEstadoDspRust();
    const exito = await invocarTauri<void>('actualizar_parametros_dsp', { nuevoEstado: estado });
    if (exito !== null) {
      setMotorRustConectado(true);
    }
  }, [construirEstadoDspRust]);

  // Ejecutar sincronizacion con debounce suave
  const temporizadorSincronizacion = useRef<NodeJS.Timeout | null>(null);
  useEffect(() => {
    if (temporizadorSincronizacion.current) {
      clearTimeout(temporizadorSincronizacion.current);
    }
    temporizadorSincronizacion.current = setTimeout(() => {
      sincronizarHaciaRust();
    }, 40);
    return () => {
      if (temporizadorSincronizacion.current) {
        clearTimeout(temporizadorSincronizacion.current);
      }
    };
  }, [sincronizarHaciaRust]);

  // Polling de telemetria en tiempo real a 30 FPS
  useEffect(() => {
    let montado = true;
    const intervalo = setInterval(async () => {
      if (!montado) return;
      const telemetria = await invocarTauri<TelemetriaUi>('obtener_telemetria_espectro');
      if (telemetria && montado) {
        setMotorRustConectado(true);
        if (Array.isArray(telemetria.barras)) {
          setBarrasEspectro(telemetria.barras);
        }
        if (Array.isArray(telemetria.puntos_curva)) {
          setPuntosCurva(telemetria.puntos_curva);
        }
        if (typeof telemetria.reduccion_compresor_db === 'number') {
          setReduccionCompresorDb(telemetria.reduccion_compresor_db);
        }
        if (typeof telemetria.pico_salida_db === 'number') {
          setPicoSalidaDb(telemetria.pico_salida_db);
        }
      }
    }, 33);

    return () => {
      montado = false;
      clearInterval(intervalo);
    };
  }, []);

  // Cargar estado inicial y dispositivos desde Tauri
  useEffect(() => {
    async function inicializar() {
      const estadoDsp = await invocarTauri<EstadoCompletoDspRust>('obtener_estado_dsp');
      if (estadoDsp) {
        setMotorRustConectado(true);
        setNombrePerfil(estadoDsp.nombre_perfil);
        setEcualizadorActivo(estadoDsp.ecualizador_activo);
        setGananciaPreampDb(estadoDsp.ganancia_preamp_db);
        setLimitadorActivo(estadoDsp.limitador_activo);
        setFrecuenciaMuestreo(estadoDsp.frecuencia_muestreo_hz);

        if (Array.isArray(estadoDsp.bandas) && estadoDsp.bandas.length > 0) {
          setBandas(
            estadoDsp.bandas.map((b) => ({
              identificador: b.identificador,
              frecuenciaHz: b.frecuencia_hz,
              factorQ: b.factor_q,
              gananciaDb: b.ganancia_db,
              tipo: b.tipo,
              activo: b.activo,
              etiqueta: formatearEtiquetaFrecuencia(b.frecuencia_hz),
              nombreHumano: formatearEtiquetaFrecuencia(b.frecuencia_hz),
              descripcion: `Filtro ${b.tipo} a ${b.frecuencia_hz} Hz`,
            }))
          );
        }

        if (estadoDsp.tono) {
          setTonoActivo(estadoDsp.tono.tono_activo);
          setGravesDb(estadoDsp.tono.graves_ganancia_db);
          setAgudosDb(estadoDsp.tono.agudos_ganancia_db);
        }

        if (estadoDsp.espacio) {
          setBalance(estadoDsp.espacio.balance);
          setExpansionEstereo(estadoDsp.espacio.expansion_estereo);
          setModoMono(estadoDsp.espacio.modo_mono);
        }

        if (estadoDsp.compresor) {
          setCompresorActivo(estadoDsp.compresor.activo);
          setCompresorLimiteDb(estadoDsp.compresor.limite_db);
          setCompresorProporcion(estadoDsp.compresor.proporcion);
          setCompresorAtaqueMs(estadoDsp.compresor.ataque_ms);
          setCompresorLanzamientoMs(estadoDsp.compresor.lanzamiento_ms);
          setCompresorSidechainHz(estadoDsp.compresor.frecuencia_sidechain_hz);
          setCompresorMaquillajeDb(estadoDsp.compresor.ganancia_maquillaje_db);
        }

        if (estadoDsp.reverberacion) {
          setReverbActivo(estadoDsp.reverberacion.activo);
          setReverbTamano(estadoDsp.reverberacion.tamano);
          setReverbPreretardoMs(estadoDsp.reverberacion.preretardo_ms);
          setReverbAmortiguacion(estadoDsp.reverberacion.amortiguacion);
          setReverbFiltroTonal(estadoDsp.reverberacion.filtro_tonal);
          setReverbMezclado(estadoDsp.reverberacion.mezclado);
        }
      }

      const listaDispositivos = await invocarTauri<DispositivoAudioWindows[]>(
        'listar_dispositivos_audio'
      );
      if (Array.isArray(listaDispositivos)) {
        setDispositivosAudio(listaDispositivos);
        const predeterminado = listaDispositivos.find((d) => d.es_predeterminado);
        if (predeterminado) {
          setDispositivoSeleccionadoId(predeterminado.identificador);
        }
      }

      // Busqueda inicial en AutoEQ
      buscarEnAutoEq('JBL');
    }

    inicializar();
  }, []);

  // Manejador para cambio de ganancia individual en un fader
  const manejarCambioGanancia = (id: number, nuevaGanancia: number) => {
    setBandas((previas) =>
      previas.map((b) => (b.identificador === id ? { ...b, gananciaDb: nuevaGanancia } : b))
    );
  };

  // Restablecer todas las bandas a plano (0 dB)
  const restablecerBandasPlanas = () => {
    setBandas((previas) => previas.map((b) => ({ ...b, gananciaDb: 0.0 })));
    setGravesDb(0.0);
    setAgudosDb(0.0);
    setGananciaPreampDb(0.0);
    setNombrePerfil('Sonido Original (Plano)');
    setMensajeEstado('Bandas ecualizadas a cero (Respuesta Plana)');
  };

  // Buscar en la base de datos de AutoEQ
  const buscarEnAutoEq = async (termino: string) => {
    setCargandoAutoEq(true);
    const resultados = await invocarTauri<ResumenPresetAutoEq[]>('buscar_auriculares_autoeq', {
      consulta: termino,
    });
    if (Array.isArray(resultados)) {
      setResultadosAutoEq(resultados);
    }
    setCargandoAutoEq(false);
  };

  // Cargar perfil seleccionado de AutoEQ
  const aplicarPerfilAutoEq = async (perfil: ResumenPresetAutoEq) => {
    setMensajeEstado(`Cargando perfil AutoEQ: ${perfil.nombre_modelo}...`);
    const nuevoEstado = await invocarTauri<EstadoCompletoDspRust>('cargar_perfil_autoeq', {
      desplazamientoBlob: perfil.desplazamiento_blob,
      nombreModelo: perfil.nombre_modelo,
    });

    if (nuevoEstado) {
      setNombrePerfil(nuevoEstado.nombre_perfil);
      setBandas(
        nuevoEstado.bandas.map((b) => ({
          identificador: b.identificador,
          frecuenciaHz: b.frecuencia_hz,
          factorQ: b.factor_q,
          gananciaDb: b.ganancia_db,
          tipo: b.tipo,
          activo: b.activo,
          etiqueta: formatearEtiquetaFrecuencia(b.frecuencia_hz),
          nombreHumano: formatearEtiquetaFrecuencia(b.frecuencia_hz),
          descripcion: `Filtro AutoEQ ${b.tipo} a ${b.frecuencia_hz} Hz`,
        }))
      );
      if (nuevoEstado.tono) {
        setGravesDb(nuevoEstado.tono.graves_ganancia_db);
        setAgudosDb(nuevoEstado.tono.agudos_ganancia_db);
      }
      setMensajeEstado(`Perfil AutoEQ [${perfil.nombre_modelo}] cargado y calibrado con exito`);
    }
  };

  // Exportar configuracion hacia Equalizer APO
  const exportarApo = async () => {
    const textoApo = await invocarTauri<string>('exportar_archivo_apo_config');
    if (textoApo) {
      setMensajeEstado('Archivo apo_config.txt exportado y sincronizado con Equalizer APO');
    }
  };

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100vh',
        width: '100vw',
        background: 'var(--fondo-espacial-0)',
        color: '#f9fafb',
        padding: '14px',
        gap: '12px',
        overflow: 'hidden',
      }}
    >
      {/* ==================================================================== */}
      {/* CABECERA SUPERIOR: Branding, Selector de Pestañas y Acciones Rápidas */}
      {/* ==================================================================== */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          background: 'var(--fondo-espacial-1)',
          padding: '10px 18px',
          borderRadius: '14px',
          border: '1px solid rgba(255,255,255,0.08)',
          backdropFilter: 'blur(16px)',
          boxShadow: '0 8px 24px rgba(0,0,0,0.5)',
        }}
      >
        {/* Identidad de Marca y Perfil Activo */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '14px' }}>
          <div
            style={{
              width: '38px',
              height: '38px',
              borderRadius: '10px',
              background: 'linear-gradient(135deg, #f59e0b 0%, #b45309 100%)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              boxShadow: '0 0 14px rgba(245, 158, 11, 0.4)',
            }}
          >
            <Sliders size={22} color="#07090e" strokeWidth={2.5} />
          </div>

          <div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <span
                style={{
                  fontFamily: 'var(--fuente-titulos)',
                  fontWeight: 800,
                  fontSize: '1.05rem',
                  letterSpacing: '0.04em',
                  color: '#ffffff',
                }}
              >
                ALDRA POWERAMP EQUALIZER
              </span>
              <span
                style={{
                  fontFamily: 'var(--fuente-datos)',
                  fontSize: '0.65rem',
                  color: 'var(--ambar-sol-brillante)',
                  background: 'rgba(245,158,11,0.15)',
                  padding: '2px 6px',
                  borderRadius: '4px',
                  border: '1px solid rgba(245,158,11,0.3)',
                  fontWeight: 700,
                }}
              >
                64-BIT DSP
              </span>
            </div>

            <div
              style={{
                fontFamily: 'var(--fuente-ui)',
                fontSize: '0.78rem',
                color: '#9ca3af',
                marginTop: '1px',
              }}
            >
              Perfil Activo:{' '}
              <span style={{ color: 'var(--ambar-sol-brillante)', fontWeight: 600 }}>
                {nombrePerfil}
              </span>
            </div>
          </div>
        </div>

        {/* Barra Central de Escenas / Pestañas Aurora */}
        <div style={{ display: 'flex', gap: '6px' }}>
          <button
            className={`boton-pestana-aurora ${vistaActiva === 'ecualizador' ? 'activo' : ''}`}
            onClick={() => setVistaActiva('ecualizador')}
          >
            <Sliders size={16} />
            ECUALIZADOR
          </button>

          <button
            className={`boton-pestana-aurora ${
              vistaActiva === 'espacio_compresor' ? 'activo' : ''
            }`}
            onClick={() => setVistaActiva('espacio_compresor')}
          >
            <Gauge size={16} />
            ESPACIO Y DINAMICA
          </button>

          <button
            className={`boton-pestana-aurora ${vistaActiva === 'reverberacion' ? 'activo' : ''}`}
            onClick={() => setVistaActiva('reverberacion')}
          >
            <Waves size={16} />
            REVERBERACION
          </button>

          <button
            className={`boton-pestana-aurora ${
              vistaActiva === 'autoeq_dispositivos' ? 'activo' : ''
            }`}
            onClick={() => setVistaActiva('autoeq_dispositivos')}
          >
            <Headphones size={16} />
            AUTOEQ Y SALIDA
          </button>
        </div>

        {/* Acciones Maestras: Encendido Global, Reset y Exportar */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          <button
            onClick={() => setEcualizadorActivo(!ecualizadorActivo)}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              padding: '8px 14px',
              borderRadius: '10px',
              border: ecualizadorActivo
                ? '1px solid rgba(16, 185, 129, 0.4)'
                : '1px solid rgba(239, 68, 68, 0.4)',
              background: ecualizadorActivo
                ? 'rgba(16, 185, 129, 0.15)'
                : 'rgba(239, 68, 68, 0.15)',
              color: ecualizadorActivo ? '#10b981' : '#ef4444',
              fontFamily: 'var(--fuente-marca)',
              fontSize: '0.78rem',
              fontWeight: 700,
              cursor: 'pointer',
            }}
          >
            <Power size={16} />
            {ecualizadorActivo ? 'ACTIVADO' : 'BYPASS'}
          </button>

          <button
            onClick={restablecerBandasPlanas}
            title="Restablecer todas las bandas a respuesta plana (0 dB)"
            style={{
              padding: '8px 12px',
              borderRadius: '10px',
              background: 'rgba(255,255,255,0.05)',
              border: '1px solid rgba(255,255,255,0.1)',
              color: '#d1d5db',
              fontFamily: 'var(--fuente-marca)',
              fontSize: '0.78rem',
              fontWeight: 600,
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
            }}
          >
            <RotateCcw size={15} />
            PLANO
          </button>

          <button
            onClick={exportarApo}
            title="Exportar archivo apo_config.txt para Equalizer APO"
            style={{
              padding: '8px 12px',
              borderRadius: '10px',
              background: 'rgba(245, 158, 11, 0.15)',
              border: '1px solid rgba(245, 158, 11, 0.35)',
              color: 'var(--ambar-sol-brillante)',
              fontFamily: 'var(--fuente-marca)',
              fontSize: '0.78rem',
              fontWeight: 700,
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
            }}
          >
            <Download size={15} />
            APO CONFIG
          </button>
        </div>
      </div>

      {/* ==================================================================== */}
      {/* CUERPO PRINCIPAL DINAMICO: Renderizado segun Escena Seleccionada      */}
      {/* ==================================================================== */}
      <div style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column' }}>
        <AnimatePresence mode="wait">
          {/* ---------------------------------------------------------------- */}
          {/* ESCENA 1: ECUALIZADOR (Faders + Curva Biquad + Espectro FFT)     */}
          {/* ---------------------------------------------------------------- */}
          {vistaActiva === 'ecualizador' && (
            <motion.div
              key="escena_ecualizador"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              transition={{ duration: 0.2 }}
              style={{
                display: 'grid',
                gridTemplateColumns: '1fr 280px',
                gap: '12px',
                height: '100%',
                minHeight: 0,
              }}
            >
              {/* Panel Izquierdo: Analizador FFT + Banco de Faders Esqueumorficos */}
              <div
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '12px',
                  height: '100%',
                  minHeight: 0,
                }}
              >
                {/* Visualizador de Espectro y Curva IIR */}
                <div style={{ height: '175px', flexShrink: 0 }}>
                  <VisualizadorEspectroAmbar
                    gananciasBandas={bandas.map((b) => b.gananciaDb)}
                    barrasEspectroTiempoReal={barrasEspectro}
                    curvaRespuesta={puntosCurva}
                  />
                </div>

                {/* Banco de 10 Faders Analógicos Esqueumórficos */}
                <TarjetaBento
                  titulo="BANCO DE ECUALIZACION PARAMETRICA / GRAFICA (64 BITS)"
                  indicador={`${bandas.length} BANDAS ISO`}
                  estiloAdicional={{ flex: 1, minHeight: 0 }}
                >
                  <div
                    style={{
                      display: 'flex',
                      justifyContent: 'space-between',
                      alignItems: 'center',
                      height: '100%',
                      padding: '4px 8px',
                      gap: '8px',
                    }}
                  >
                    {bandas.map((banda) => (
                      <FaderEsqueumorfico
                        key={banda.identificador}
                        banda={banda}
                        alCambiarGanancia={manejarCambioGanancia}
                      />
                    ))}
                  </div>
                </TarjetaBento>
              </div>

              {/* Panel Derecho: Controles Maestros de Tono, Preamp y Limitador */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                {/* Controles de Tono (Graves y Agudos) */}
                <TarjetaBento
                  titulo="CONTROLES DE TONO"
                  indicador={tonoActivo ? 'ACTIVO' : 'BYPASS'}
                >
                  <div
                    style={{
                      display: 'flex',
                      justifyContent: 'space-around',
                      alignItems: 'center',
                      padding: '12px 0',
                    }}
                  >
                    <PerillaRotativa3D
                      etiqueta="GRAVES"
                      valor={gravesDb}
                      minimo={-12.0}
                      maximo={12.0}
                      unidad="dB"
                      descripcion="Refuerzo de estante bajo a 90 Hz"
                      alCambiarValor={setGravesDb}
                    />

                    <PerillaRotativa3D
                      etiqueta="AGUDOS"
                      valor={agudosDb}
                      minimo={-12.0}
                      maximo={12.0}
                      unidad="dB"
                      descripcion="Refuerzo de estante alto a 10 kHz"
                      alCambiarValor={setAgudosDb}
                    />
                  </div>

                  <div style={{ marginTop: '8px', display: 'flex', justifyContent: 'center' }}>
                    <button
                      onClick={() => setTonoActivo(!tonoActivo)}
                      style={{
                        padding: '6px 14px',
                        borderRadius: '8px',
                        background: tonoActivo
                          ? 'rgba(245,158,11,0.15)'
                          : 'rgba(255,255,255,0.05)',
                        border: tonoActivo
                          ? '1px solid rgba(245,158,11,0.35)'
                          : '1px solid rgba(255,255,255,0.1)',
                        color: tonoActivo ? 'var(--ambar-sol-brillante)' : '#9ca3af',
                        fontFamily: 'var(--fuente-marca)',
                        fontSize: '0.74rem',
                        fontWeight: 700,
                        cursor: 'pointer',
                      }}
                    >
                      {tonoActivo ? 'TONO ENCENDIDO' : 'TONO APAGADO'}
                    </button>
                  </div>
                </TarjetaBento>

                {/* Preamplificador y Limitador Anti-Clipping */}
                <TarjetaBento
                  titulo="PREAMPLIFICADOR Y LIMITADOR"
                  indicador="PROTECCION"
                  estiloAdicional={{ flex: 1 }}
                >
                  <div
                    style={{
                      display: 'flex',
                      flexDirection: 'column',
                      alignItems: 'center',
                      gap: '16px',
                      padding: '12px 0',
                    }}
                  >
                    <PerillaRotativa3D
                      etiqueta="PREAMP"
                      valor={gananciaPreampDb}
                      minimo={-15.0}
                      maximo={15.0}
                      unidad="dB"
                      descripcion="Ganancia general antes del procesamiento Biquad"
                      alCambiarValor={setGananciaPreampDb}
                    />

                    <div
                      style={{
                        width: '100%',
                        background: 'rgba(0,0,0,0.3)',
                        padding: '12px',
                        borderRadius: '12px',
                        border: '1px solid rgba(255,255,255,0.06)',
                        display: 'flex',
                        flexDirection: 'column',
                        gap: '8px',
                      }}
                    >
                      <div
                        style={{
                          display: 'flex',
                          justifyContent: 'space-between',
                          alignItems: 'center',
                        }}
                      >
                        <span style={{ fontSize: '0.76rem', color: '#d1d5db', fontWeight: 600 }}>
                          Limitador Maestro:
                        </span>
                        <button
                          onClick={() => setLimitadorActivo(!limitadorActivo)}
                          style={{
                            padding: '4px 10px',
                            borderRadius: '6px',
                            background: limitadorActivo
                              ? 'rgba(16,185,129,0.2)'
                              : 'rgba(255,255,255,0.05)',
                            border: limitadorActivo
                              ? '1px solid rgba(16,185,129,0.4)'
                              : '1px solid rgba(255,255,255,0.1)',
                            color: limitadorActivo ? '#10b981' : '#9ca3af',
                            fontFamily: 'var(--fuente-datos)',
                            fontSize: '0.70rem',
                            fontWeight: 700,
                            cursor: 'pointer',
                          }}
                        >
                          {limitadorActivo ? 'ACTIVO' : 'BYPASS'}
                        </button>
                      </div>

                      <div
                        style={{
                          display: 'flex',
                          justifyContent: 'space-between',
                          alignItems: 'center',
                        }}
                      >
                        <span style={{ fontSize: '0.76rem', color: '#d1d5db', fontWeight: 600 }}>
                          Pico de Salida:
                        </span>
                        <span
                          style={{
                            fontFamily: 'var(--fuente-datos)',
                            fontSize: '0.80rem',
                            color: picoSalidaDb > -1.0 ? '#ef4444' : '#10b981',
                            fontWeight: 700,
                          }}
                        >
                          {picoSalidaDb.toFixed(1)} dBFS
                        </span>
                      </div>
                    </div>
                  </div>
                </TarjetaBento>
              </div>
            </motion.div>
          )}

          {/* ---------------------------------------------------------------- */}
          {/* ESCENA 2: ESPACIO Y DINAMICA (Compresor sin tempo + Mid/Side)     */}
          {/* ---------------------------------------------------------------- */}
          {vistaActiva === 'espacio_compresor' && (
            <motion.div
              key="escena_espacio"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              transition={{ duration: 0.2 }}
              style={{
                display: 'grid',
                gridTemplateColumns: '360px 1fr',
                gap: '12px',
                height: '100%',
                minHeight: 0,
              }}
            >
              {/* Procesador de Espacio (Mid/Side Stereo, Balance, Mono) */}
              <TarjetaBento
                titulo="PROCESADOR DE ESPACIO ESTEREO"
                indicador="MID / SIDE"
              >
                <div
                  style={{
                    display: 'flex',
                    flexDirection: 'column',
                    alignItems: 'center',
                    gap: '24px',
                    padding: '20px 0',
                    flex: 1,
                    justifyContent: 'center',
                  }}
                >
                  <PerillaRotativa3D
                    etiqueta="EXPANSION ESTEREO"
                    valor={expansionEstereo}
                    minimo={0.0}
                    maximo={1.0}
                    unidad=""
                    descripcion="Amplitud estereofonica Mid/Side de 0% a 200%"
                    alCambiarValor={setExpansionEstereo}
                  />

                  <PerillaRotativa3D
                    etiqueta="BALANCE L / R"
                    valor={balance}
                    minimo={-1.0}
                    maximo={1.0}
                    unidad=""
                    descripcion="Atenuacion paneada entre canales Izquierdo y Derecho"
                    alCambiarValor={setBalance}
                  />

                  <div
                    style={{
                      width: '100%',
                      background: 'rgba(0,0,0,0.3)',
                      padding: '14px',
                      borderRadius: '12px',
                      border: '1px solid rgba(255,255,255,0.06)',
                      display: 'flex',
                      justifyContent: 'space-between',
                      alignItems: 'center',
                    }}
                  >
                    <div>
                      <div style={{ fontSize: '0.82rem', fontWeight: 700, color: '#ffffff' }}>
                        Modo Monofonico
                      </div>
                      <div style={{ fontSize: '0.70rem', color: '#9ca3af' }}>
                        Mezcla L+R equilibrada
                      </div>
                    </div>

                    <button
                      onClick={() => setModoMono(!modoMono)}
                      style={{
                        padding: '6px 14px',
                        borderRadius: '8px',
                        background: modoMono
                          ? 'rgba(245,158,11,0.2)'
                          : 'rgba(255,255,255,0.05)',
                        border: modoMono
                          ? '1px solid rgba(245,158,11,0.4)'
                          : '1px solid rgba(255,255,255,0.1)',
                        color: modoMono ? 'var(--ambar-sol-brillante)' : '#9ca3af',
                        fontFamily: 'var(--fuente-marca)',
                        fontSize: '0.76rem',
                        fontWeight: 700,
                        cursor: 'pointer',
                      }}
                    >
                      {modoMono ? 'MONO ACTIVO' : 'ESTEREO'}
                    </button>
                  </div>
                </div>
              </TarjetaBento>

              {/* Compresor Dinamico de Precision estilo Poweramp */}
              <TarjetaBento
                titulo="COMPRESOR DINAMICO ESTEREO CON SIDECHAIN"
                indicador={compresorActivo ? 'COMPRESION ACTIVA' : 'BYPASS'}
              >
                <div
                  style={{
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '20px',
                    height: '100%',
                    justifyContent: 'space-between',
                  }}
                >
                  {/* Medidor de Reduccion de Ganancia en Vivo */}
                  <div
                    style={{
                      background: 'rgba(0,0,0,0.4)',
                      padding: '12px 18px',
                      borderRadius: '12px',
                      border: '1px solid rgba(255,255,255,0.06)',
                      display: 'flex',
                      justifyContent: 'space-between',
                      alignItems: 'center',
                    }}
                  >
                    <div>
                      <span style={{ fontSize: '0.74rem', color: '#9ca3af', fontWeight: 600 }}>
                        REDUCCION DE GANANCIA (GR)
                      </span>
                      <div
                        style={{
                          fontFamily: 'var(--fuente-datos)',
                          fontSize: '1.2rem',
                          color: 'var(--ambar-sol-brillante)',
                          fontWeight: 800,
                        }}
                      >
                        -{reduccionCompresorDb.toFixed(2)} dB
                      </div>
                    </div>

                    {/* Barra VU Visual de Reduccion */}
                    <div
                      style={{
                        flex: 1,
                        maxWidth: '320px',
                        height: '14px',
                        background: '#090c12',
                        borderRadius: '999px',
                        overflow: 'hidden',
                        position: 'relative',
                        border: '1px solid rgba(255,255,255,0.08)',
                      }}
                    >
                      <div
                        style={{
                          height: '100%',
                          width: `${Math.min(100, (reduccionCompresorDb / 18.0) * 100)}%`,
                          background: 'linear-gradient(90deg, #f59e0b 0%, #ff0055 100%)',
                          boxShadow: '0 0 10px rgba(245,158,11,0.5)',
                          transition: 'width 0.08s ease-out',
                        }}
                      />
                    </div>

                    <button
                      onClick={() => setCompresorActivo(!compresorActivo)}
                      style={{
                        padding: '6px 14px',
                        borderRadius: '8px',
                        background: compresorActivo
                          ? 'rgba(16,185,129,0.18)'
                          : 'rgba(255,255,255,0.05)',
                        border: compresorActivo
                          ? '1px solid rgba(16,185,129,0.35)'
                          : '1px solid rgba(255,255,255,0.1)',
                        color: compresorActivo ? '#10b981' : '#9ca3af',
                        fontFamily: 'var(--fuente-marca)',
                        fontSize: '0.74rem',
                        fontWeight: 700,
                        cursor: 'pointer',
                      }}
                    >
                      {compresorActivo ? 'ACTIVADO' : 'APAGADO'}
                    </button>
                  </div>

                  {/* Perillas del Compresor (Umbral, Ratio, Ataque, Lanzamiento, Sidechain, Maquillaje) */}
                  <div
                    style={{
                      display: 'grid',
                      gridTemplateColumns: 'repeat(6, 1fr)',
                      gap: '12px',
                      alignItems: 'center',
                      padding: '10px 0',
                    }}
                  >
                    <PerillaRotativa3D
                      etiqueta="UMBRAL"
                      valor={compresorLimiteDb}
                      minimo={-60.0}
                      maximo={0.0}
                      unidad="dB"
                      descripcion="Punto a partir del cual comienza a atenuar la senal"
                      alCambiarValor={setCompresorLimiteDb}
                    />

                    <PerillaRotativa3D
                      etiqueta="PROPORCION"
                      valor={compresorProporcion}
                      minimo={1.0}
                      maximo={20.0}
                      unidad=":1"
                      descripcion="Relacion de compresion (ejemplo 4:1)"
                      alCambiarValor={setCompresorProporcion}
                    />

                    <PerillaRotativa3D
                      etiqueta="ATAQUE"
                      valor={compresorAtaqueMs}
                      minimo={0.1}
                      maximo={100.0}
                      unidad="ms"
                      descripcion="Velocidad de respuesta ante transitorios"
                      alCambiarValor={setCompresorAtaqueMs}
                    />

                    <PerillaRotativa3D
                      etiqueta="LIBERACION"
                      valor={compresorLanzamientoMs}
                      minimo={10.0}
                      maximo={800.0}
                      unidad="ms"
                      descripcion="Tiempo de recuperacion tras el pico"
                      alCambiarValor={setCompresorLanzamientoMs}
                    />

                    <PerillaRotativa3D
                      etiqueta="SIDECHAIN"
                      valor={compresorSidechainHz}
                      minimo={20.0}
                      maximo={500.0}
                      unidad="Hz"
                      descripcion="Filtro paso alto en la senal de deteccion (no ahoga los bombos)"
                      alCambiarValor={setCompresorSidechainHz}
                    />

                    <PerillaRotativa3D
                      etiqueta="GANANCIA"
                      valor={compresorMaquillajeDb}
                      minimo={-6.0}
                      maximo={18.0}
                      unidad="dB"
                      descripcion="Ganancia de compensacion de nivel de salida"
                      alCambiarValor={setCompresorMaquillajeDb}
                    />
                  </div>

                  <div
                    style={{
                      fontFamily: 'var(--fuente-datos)',
                      fontSize: '0.72rem',
                      color: '#9ca3af',
                      textAlign: 'center',
                    }}
                  >
                    Algoritmo de rodilla suave (Soft Knee) con deteccion RMS y envolvente de 64 bits.
                  </div>
                </div>
              </TarjetaBento>
            </motion.div>
          )}

          {/* ---------------------------------------------------------------- */}
          {/* ESCENA 3: REVERBERACION ESTEREO (Schroeder-Moorer / Freeverb)     */}
          {/* ---------------------------------------------------------------- */}
          {vistaActiva === 'reverberacion' && (
            <motion.div
              key="escena_reverb"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              transition={{ duration: 0.2 }}
              style={{ height: '100%', minHeight: 0 }}
            >
              <TarjetaBento
                titulo="PROCESADOR DE REVERBERACION ALGORITMICA ESTEREO"
                indicador={reverbActivo ? 'ESPACIALIDAD ACTIVA' : 'BYPASS'}
                estiloAdicional={{ height: '100%', justifyContent: 'space-between' }}
              >
                <div
                  style={{
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '30px',
                    alignItems: 'center',
                    padding: '24px 0',
                    flex: 1,
                    justifyContent: 'center',
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: '14px' }}>
                    <button
                      onClick={() => setReverbActivo(!reverbActivo)}
                      style={{
                        padding: '10px 24px',
                        borderRadius: '12px',
                        background: reverbActivo
                          ? 'rgba(245,158,11,0.22)'
                          : 'rgba(255,255,255,0.05)',
                        border: reverbActivo
                          ? '1px solid rgba(245,158,11,0.45)'
                          : '1px solid rgba(255,255,255,0.1)',
                        color: reverbActivo ? 'var(--ambar-sol-brillante)' : '#9ca3af',
                        fontFamily: 'var(--fuente-marca)',
                        fontSize: '0.86rem',
                        fontWeight: 800,
                        cursor: 'pointer',
                        boxShadow: reverbActivo ? '0 0 16px rgba(245,158,11,0.25)' : 'none',
                      }}
                    >
                      {reverbActivo ? 'REVERBERACION ENCENDIDA' : 'REVERBERACION APAGADA'}
                    </button>
                  </div>

                  {/* Perillas de Sala */}
                  <div
                    style={{
                      display: 'grid',
                      gridTemplateColumns: 'repeat(5, 1fr)',
                      gap: '24px',
                      maxWidth: '850px',
                      width: '100%',
                    }}
                  >
                    <PerillaRotativa3D
                      etiqueta="TAMANO SALA"
                      valor={reverbTamano}
                      minimo={0.0}
                      maximo={1.0}
                      unidad=""
                      descripcion="Dimensiones acusticas del recinto modelado"
                      alCambiarValor={setReverbTamano}
                    />

                    <PerillaRotativa3D
                      etiqueta="PRERETARDO"
                      valor={reverbPreretardoMs}
                      minimo={0.0}
                      maximo={100.0}
                      unidad="ms"
                      descripcion="Tiempo entre la senal directa y las primeras reflexiones"
                      alCambiarValor={setReverbPreretardoMs}
                    />

                    <PerillaRotativa3D
                      etiqueta="AMORTIGUACION"
                      valor={reverbAmortiguacion}
                      minimo={0.0}
                      maximo={1.0}
                      unidad=""
                      descripcion="Absorcion de altas frecuencias por las paredes"
                      alCambiarValor={setReverbAmortiguacion}
                    />

                    <PerillaRotativa3D
                      etiqueta="FILTRO TONAL"
                      valor={reverbFiltroTonal}
                      minimo={0.0}
                      maximo={1.0}
                      unidad=""
                      descripcion="Coloracion acustica de la cola (0 = calido, 1 = brillante)"
                      alCambiarValor={setReverbFiltroTonal}
                    />

                    <PerillaRotativa3D
                      etiqueta="MEZCLADO (WET)"
                      valor={reverbMezclado}
                      minimo={0.0}
                      maximo={1.0}
                      unidad=""
                      descripcion="Proporcion de senal con reverberacion frente a senal pura"
                      alCambiarValor={setReverbMezclado}
                    />
                  </div>

                  <div
                    style={{
                      background: 'rgba(0,0,0,0.3)',
                      padding: '12px 20px',
                      borderRadius: '12px',
                      border: '1px solid rgba(255,255,255,0.06)',
                      color: '#9ca3af',
                      fontFamily: 'var(--fuente-datos)',
                      fontSize: '0.74rem',
                      maxWidth: '700px',
                      textAlign: 'center',
                    }}
                  >
                    Modelo acustico Schroeder-Moorer con 8 filtros de peine paralelos y 4 filtros
                    pasa-todo en cascada estéreo.
                  </div>
                </div>
              </TarjetaBento>
            </motion.div>
          )}

          {/* ---------------------------------------------------------------- */}
          {/* ESCENA 4: AUTOEQ Y DISPOSITIVOS WASAPI                            */}
          {/* ---------------------------------------------------------------- */}
          {vistaActiva === 'autoeq_dispositivos' && (
            <motion.div
              key="escena_autoeq"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              transition={{ duration: 0.2 }}
              style={{
                display: 'grid',
                gridTemplateColumns: '1fr 340px',
                gap: '12px',
                height: '100%',
                minHeight: 0,
              }}
            >
              {/* Buscador de Perfiles AutoEQ (+5400 modelos) */}
              <TarjetaBento
                titulo="CATALOGO AUTOEQ (+5,400 AUDIFONOS INDEXADOS)"
                indicador={`${resultadosAutoEq.length} RESULTADOS`}
                estiloAdicional={{ height: '100%', minHeight: 0 }}
              >
                <div
                  style={{
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '12px',
                    height: '100%',
                    minHeight: 0,
                  }}
                >
                  {/* Entrada de Busqueda */}
                  <div style={{ display: 'flex', gap: '8px' }}>
                    <div
                      style={{
                        flex: 1,
                        display: 'flex',
                        alignItems: 'center',
                        gap: '8px',
                        background: '#07090e',
                        padding: '8px 14px',
                        borderRadius: '10px',
                        border: '1px solid rgba(255,255,255,0.1)',
                      }}
                    >
                      <Search size={16} color="#9ca3af" />
                      <input
                        type="text"
                        value={consultaAutoEq}
                        onChange={(e) => {
                          setConsultaAutoEq(e.target.value);
                          buscarEnAutoEq(e.target.value);
                        }}
                        placeholder="Escribe marca o modelo (ej. Sony WH-1000XM4, JBL Tune, Sennheiser, AirPods)..."
                        style={{
                          background: 'transparent',
                          border: 'none',
                          color: '#ffffff',
                          fontFamily: 'var(--fuente-ui)',
                          fontSize: '0.82rem',
                          outline: 'none',
                          width: '100%',
                        }}
                      />
                    </div>
                  </div>

                  {/* Lista de Resultados con Scroll */}
                  <div
                    style={{
                      flex: 1,
                      minHeight: 0,
                      overflowY: 'auto',
                      display: 'flex',
                      flexDirection: 'column',
                      gap: '6px',
                      paddingRight: '4px',
                    }}
                  >
                    {cargandoAutoEq ? (
                      <div
                        style={{
                          textAlign: 'center',
                          padding: '30px',
                          color: '#9ca3af',
                          fontSize: '0.82rem',
                        }}
                      >
                        Buscando en auto_eq.db...
                      </div>
                    ) : resultadosAutoEq.length === 0 ? (
                      <div
                        style={{
                          textAlign: 'center',
                          padding: '30px',
                          color: '#9ca3af',
                          fontSize: '0.82rem',
                        }}
                      >
                        No se encontraron modelos coincidentes en auto_eq.db
                      </div>
                    ) : (
                      resultadosAutoEq.map((item) => (
                        <div
                          key={item.desplazamiento_blob}
                          onClick={() => aplicarPerfilAutoEq(item)}
                          style={{
                            display: 'flex',
                            justifyContent: 'space-between',
                            alignItems: 'center',
                            background:
                              nombrePerfil === item.nombre_modelo
                                ? 'rgba(245,158,11,0.18)'
                                : 'rgba(255,255,255,0.03)',
                            padding: '10px 14px',
                            borderRadius: '10px',
                            border:
                              nombrePerfil === item.nombre_modelo
                                ? '1px solid rgba(245,158,11,0.45)'
                                : '1px solid rgba(255,255,255,0.05)',
                            cursor: 'pointer',
                            transition: 'all 0.15s ease',
                          }}
                        >
                          <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                            <Headphones
                              size={18}
                              color={
                                nombrePerfil === item.nombre_modelo
                                  ? 'var(--ambar-sol-brillante)'
                                  : '#9ca3af'
                              }
                            />
                            <div>
                              <div
                                style={{
                                  fontSize: '0.82rem',
                                  fontWeight: 600,
                                  color: '#ffffff',
                                }}
                              >
                                {item.nombre_modelo}
                              </div>
                              <div
                                style={{
                                  fontFamily: 'var(--fuente-datos)',
                                  fontSize: '0.68rem',
                                  color: '#9ca3af',
                                }}
                              >
                                {item.numero_bandas} bandas |{' '}
                                {item.es_parametrico ? 'Parametrico' : 'Grafico'}
                              </div>
                            </div>
                          </div>

                          {nombrePerfil === item.nombre_modelo ? (
                            <span
                              style={{
                                color: 'var(--ambar-sol-brillante)',
                                display: 'flex',
                                alignItems: 'center',
                                gap: '4px',
                                fontSize: '0.74rem',
                                fontWeight: 700,
                              }}
                            >
                              <CheckCircle2 size={16} /> ACTIVO
                            </span>
                          ) : (
                            <span
                              style={{
                                fontSize: '0.72rem',
                                color: '#d1d5db',
                                background: 'rgba(255,255,255,0.08)',
                                padding: '4px 8px',
                                borderRadius: '6px',
                              }}
                            >
                              APLICAR
                            </span>
                          )}
                        </div>
                      ))
                    )}
                  </div>
                </div>
              </TarjetaBento>

              {/* Selector de Dispositivos WASAPI */}
              <TarjetaBento
                titulo="DISPOSITIVOS DE SALIDA WINDOWS"
                indicador="WASAPI"
                estiloAdicional={{ height: '100%', minHeight: 0 }}
              >
                <div
                  style={{
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '10px',
                    height: '100%',
                    minHeight: 0,
                  }}
                >
                  <span style={{ fontSize: '0.72rem', color: '#9ca3af' }}>
                    Selecciona los audifonos, altavoces o DAC donde se enrutara el audio:
                  </span>

                  <div
                    style={{
                      flex: 1,
                      minHeight: 0,
                      overflowY: 'auto',
                      display: 'flex',
                      flexDirection: 'column',
                      gap: '8px',
                    }}
                  >
                    {dispositivosAudio.map((disp) => {
                      const esSeleccionado = disp.identificador === dispositivoSeleccionadoId;
                      return (
                        <div
                          key={disp.identificador}
                          onClick={async () => {
                            setDispositivoSeleccionadoId(disp.identificador);
                            await invocarTauri('seleccionar_dispositivo_salida', {
                              idDispositivo: disp.identificador,
                            });
                            setMensajeEstado(
                              `Dispositivo de salida seleccionado: ${disp.nombre_amigable}`
                            );
                          }}
                          style={{
                            padding: '10px 12px',
                            borderRadius: '10px',
                            background: esSeleccionado
                              ? 'rgba(16,185,129,0.18)'
                              : 'rgba(255,255,255,0.03)',
                            border: esSeleccionado
                              ? '1px solid rgba(16,185,129,0.45)'
                              : '1px solid rgba(255,255,255,0.06)',
                            cursor: 'pointer',
                            display: 'flex',
                            alignItems: 'center',
                            gap: '10px',
                          }}
                        >
                          <Speaker
                            size={18}
                            color={esSeleccionado ? '#10b981' : '#9ca3af'}
                          />
                          <div style={{ flex: 1 }}>
                            <div
                              style={{
                                fontSize: '0.78rem',
                                fontWeight: 600,
                                color: '#ffffff',
                              }}
                            >
                              {disp.nombre_amigable}
                            </div>
                            {disp.es_predeterminado && (
                              <div
                                style={{
                                  fontSize: '0.66rem',
                                  color: 'var(--ambar-sol-brillante)',
                                  fontWeight: 700,
                                }}
                              >
                                PREDETERMINADO EN WINDOWS
                              </div>
                            )}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </div>
              </TarjetaBento>
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      {/* ==================================================================== */}
      {/* BARRA INFERIOR DE ESTADO Y TELEMETRIA                                 */}
      {/* ==================================================================== */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          background: 'var(--fondo-espacial-1)',
          padding: '8px 16px',
          borderRadius: '12px',
          border: '1px solid rgba(255,255,255,0.06)',
          fontSize: '0.74rem',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <Cpu
            size={16}
            color={motorRustConectado ? 'var(--ambar-sol-brillante)' : '#10b981'}
          />
          <span style={{ color: '#ffffff', fontWeight: 600 }}>{mensajeEstado}</span>
        </div>

        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '16px',
            fontFamily: 'var(--fuente-datos)',
            color: '#9ca3af',
          }}
        >
          <span>Fs: {frecuenciaMuestreo} Hz</span>
          <span>Reduccion: -{reduccionCompresorDb.toFixed(1)} dB</span>
          <span>Pico: {picoSalidaDb.toFixed(1)} dBFS</span>
          <span
            style={{
              color: motorRustConectado ? '#10b981' : '#f59e0b',
              fontWeight: 700,
            }}
          >
            {motorRustConectado ? 'TAURI IPC ACTIVO' : 'MODO LOCAL'}
          </span>
        </div>
      </div>
    </div>
  );
}
