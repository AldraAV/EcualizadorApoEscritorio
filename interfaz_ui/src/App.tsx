import React, { useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import {
  Activity,
  Cpu,
  FolderOpen,
  Gauge,
  Headphones,
  Plus,
  Power,
  Radio,
  RotateCcw,
  Search,
  ShieldCheck,
  Sliders,
  Trash2,
  Waves,
} from 'lucide-react';
import { TarjetaBento } from './componentes/TarjetaBento';
import { FaderEsqueumorfico } from './componentes/FaderEsqueumorfico';
import { PerillaRotativa3D } from './componentes/PerillaRotativa3D';
import { VisualizadorEspectroAmbar } from './componentes/VisualizadorEspectroAmbar';
import {
  BandaAudio,
  EstadoEcualizador,
  PuntoCurvaRespuesta,
  ResumenPresetAutoEqUi,
  TipoFiltro,
  VistaActivaPoweramp,
} from './tipos/audio';

const URL_MOTOR_RUST = 'http://127.0.0.1:18765';

const BANDAS_INICIALES: BandaAudio[] = [
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
    descripcion: 'Golpe y plenitud de guitarras graves y chelos',
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
    descripcion: 'Tono calido de voces graves y percusion media',
  },
  {
    identificador: 5,
    frecuenciaHz: 500,
    factorQ: 1.41,
    gananciaDb: -1.2,
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
    descripcion: 'Ataque de bateria, tarola y detalles de cuerdas',
  },
  {
    identificador: 9,
    frecuenciaHz: 8000,
    factorQ: 1.41,
    gananciaDb: 4.5,
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

export const App: React.FC = () => {
  const [estado, setEstado] = useState<EstadoEcualizador>({
    frecuenciaMuestreo: 48000,
    gananciaPreampDb: 0.0,
    limitadorActivo: true,
    compresorUmbralDb: -6.0,
    compresorRatio: 2.5,
    perfilActual: 'Potencia Analogica (Poweramp)',
    modoParametrico: true,
    bandas: BANDAS_INICIALES,
    tono: {
      tonoActivo: true,
      gravesGananciaDb: 0.0,
      gravesFrecuenciaHz: 90.0,
      gravesFactorQ: 0.8,
      agudosGananciaDb: 0.0,
      agudosFrecuenciaHz: 10000.0,
      agudosFactorQ: 0.6,
    },
    espacio: {
      balance: 0.0,
      expansionEstereo: 0.0,
      modoMono: false,
      tempo: 1.0,
    },
    compresor: {
      activo: true,
      limiteDb: -6.0,
      proporcion: 2.5,
      ataqueMs: 10.0,
      lanzamientoMs: 100.0,
      frecuenciaSidechainHz: 90.0,
      gananciaMaquillajeDb: 0.0,
    },
    reverberacion: {
      activo: false,
      tamano: 0.5,
      preretardoMs: 15.0,
      amortiguacion: 0.5,
      filtroTonal: 0.5,
      mezclado: 0.2,
    },
  });

  const [vistaActiva, setVistaActiva] = useState<VistaActivaPoweramp>('ecualizador');
  const [encendido, setEncendido] = useState<boolean>(true);
  const [bandaSeleccionadaId, setBandaSeleccionadaId] = useState<number>(1);
  const [motorRustConectado, setMotorRustConectado] = useState<boolean>(false);
  const [barrasEspectroVivo, setBarrasEspectroVivo] = useState<number[]>([]);
  const [curvaRespuestaIir, setCurvaRespuestaIir] = useState<PuntoCurvaRespuesta[]>([]);
  const [reduccionCompresorVivoDb, setReduccionCompresorVivoDb] = useState<number>(0.0);
  const [picoSalidaVivoDb, setPicoSalidaVivoDb] = useState<number>(-18.0);
  const [consultaAutoEq, setConsultaAutoEq] = useState<string>('JBL');
  const [resultadosAutoEq, setResultadosAutoEq] = useState<ResumenPresetAutoEqUi[]>([]);
  const [mensajeEstado, setMensajeEstado] = useState<string>(
    'Motor DSP Rust (64 bits) sincronizado con Equalizer APO'
  );

  // Sincronizar respuesta JSON del servidor Rust con el estado React
  const aplicarRespuestaMotorRust = (datos: any) => {
    if (!datos || !Array.isArray(datos.bandas)) return;

    const bandasMapeadas: BandaAudio[] = datos.bandas.map((b: any) => ({
      identificador: b.identificador,
      frecuenciaHz: b.frecuencia_hz,
      factorQ: b.factor_q,
      gananciaDb: b.ganancia_db,
      tipo: (b.tipo as TipoFiltro) || 'PicoParametrico',
      activo: Boolean(b.activo),
      etiqueta: formatearEtiquetaFrecuencia(b.frecuencia_hz),
      nombreHumano: formatearEtiquetaFrecuencia(b.frecuencia_hz),
    }));

    setEncendido(Boolean(datos.ecualizador_activo));
    if (Array.isArray(datos.espectro_barras)) {
      setBarrasEspectroVivo(datos.espectro_barras);
    }
    if (Array.isArray(datos.curva_respuesta)) {
      setCurvaRespuestaIir(
        datos.curva_respuesta.map((p: any) => ({
          frecuenciaHz: p.frecuencia_hz,
          gananciaDb: p.ganancia_db,
        }))
      );
    }
    if (typeof datos.reduccion_compresor_db === 'number') {
      setReduccionCompresorVivoDb(datos.reduccion_compresor_db);
    }
    if (typeof datos.pico_salida_db === 'number') {
      setPicoSalidaVivoDb(datos.pico_salida_db);
    }

    setEstado((previo) => ({
      ...previo,
      perfilActual: datos.nombre_perfil || previo.perfilActual,
      frecuenciaMuestreo: datos.frecuencia_muestreo_hz || 48000,
      gananciaPreampDb: datos.ganancia_preamp_db ?? previo.gananciaPreampDb,
      limitadorActivo: datos.limitador_activo ?? previo.limitadorActivo,
      modoParametrico: datos.modo_parametrico ?? true,
      bandas: bandasMapeadas,
      tono: datos.tono
        ? {
            tonoActivo: datos.tono.tono_activo,
            gravesGananciaDb: datos.tono.graves_ganancia_db,
            gravesFrecuenciaHz: datos.tono.graves_frecuencia_hz,
            gravesFactorQ: datos.tono.graves_factor_q,
            agudosGananciaDb: datos.tono.agudos_ganancia_db,
            agudosFrecuenciaHz: datos.tono.agudos_frecuencia_hz,
            agudosFactorQ: datos.tono.agudos_factor_q,
          }
        : previo.tono,
      espacio: datos.espacio
        ? {
            balance: datos.espacio.balance,
            expansionEstereo: datos.espacio.expansion_estereo,
            modoMono: datos.espacio.modo_mono,
            tempo: datos.espacio.tempo,
          }
        : previo.espacio,
      compresor: datos.compresor
        ? {
            activo: datos.compresor.activo,
            limiteDb: datos.compresor.limite_db,
            proporcion: datos.compresor.proporcion,
            ataqueMs: datos.compresor.ataque_ms,
            lanzamientoMs: datos.compresor.lanzamiento_ms,
            frecuenciaSidechainHz: datos.compresor.frecuencia_sidechain_hz,
            gananciaMaquillajeDb: datos.compresor.ganancia_maquillaje_db,
          }
        : previo.compresor,
      reverberacion: datos.reverberacion
        ? {
            activo: datos.reverberacion.activo,
            tamano: datos.reverberacion.tamano,
            preretardoMs: datos.reverberacion.preretardo_ms,
            amortiguacion: datos.reverberacion.amortiguacion,
            filtroTonal: datos.reverberacion.filtro_tonal,
            mezclado: datos.reverberacion.mezclado,
          }
        : previo.reverberacion,
    }));
  };

  // Enviar comando al motor Rust local
  const enviarComandoMotorRust = async (comandoTexto: string) => {
    try {
      const respuesta = await fetch(`${URL_MOTOR_RUST}/comando`, {
        method: 'POST',
        headers: { 'Content-Type': 'text/plain;charset=utf-8' },
        body: comandoTexto,
      });
      if (respuesta.ok) {
        const datos = await respuesta.json();
        setMotorRustConectado(true);
        aplicarRespuestaMotorRust(datos);
      }
    } catch {
      setMotorRustConectado(false);
    }
  };

  // Consultar modelos de audifonos en auto_eq.db a traves del motor Rust
  const buscarEnBaseDatosAutoEq = async (termino: string) => {
    try {
      const respuesta = await fetch(
        `${URL_MOTOR_RUST}/autoeq?q=${encodeURIComponent(termino)}`
      );
      if (respuesta.ok) {
        const datos = await respuesta.json();
        if (Array.isArray(datos.resultados)) {
          setResultadosAutoEq(
            datos.resultados.map((r: any) => ({
              indice: r.indice,
              nombreModelo: r.nombre_modelo,
              esParametrico: Boolean(r.es_parametrico),
              numeroBandas: r.numero_bandas,
              desplazamientoBlob: r.desplazamiento_blob,
            }))
          );
        }
      }
    } catch {
      // Sin conexion al daemon HTTP local
    }
  };

  // Conexion inicial y bucle de telemetria de espectro FFT
  useEffect(() => {
    let cancelado = false;

    const cargarEstadoInicial = async () => {
      try {
        const resp = await fetch(`${URL_MOTOR_RUST}/estado`);
        if (resp.ok && !cancelado) {
          const datos = await resp.json();
          setMotorRustConectado(true);
          aplicarRespuestaMotorRust(datos);
          buscarEnBaseDatosAutoEq('JBL');
        }
      } catch {
        if (!cancelado) setMotorRustConectado(false);
      }
    };

    cargarEstadoInicial();

    const intervalo = setInterval(async () => {
      try {
        const resp = await fetch(`${URL_MOTOR_RUST}/espectro`);
        if (resp.ok && !cancelado) {
          const datos = await resp.json();
          setMotorRustConectado(true);
          if (Array.isArray(datos.espectro_barras)) {
            setBarrasEspectroVivo(datos.espectro_barras);
          }
          if (typeof datos.reduccion_compresor_db === 'number') {
            setReduccionCompresorVivoDb(datos.reduccion_compresor_db);
          }
          if (typeof datos.pico_salida_db === 'number') {
            setPicoSalidaVivoDb(datos.pico_salida_db);
          }
        }
      } catch {
        if (!cancelado) setMotorRustConectado(false);
      }
    }, 80);

    return () => {
      cancelado = true;
      clearInterval(intervalo);
    };
  }, []);

  const manejarCambioGananciaBanda = (identificador: number, nuevaGanancia: number) => {
    setBandaSeleccionadaId(identificador);
    setEstado((previo) => ({
      ...previo,
      bandas: previo.bandas.map((b) =>
        b.identificador === identificador ? { ...b, gananciaDb: nuevaGanancia } : b
      ),
    }));
    enviarComandoMotorRust(`BANDA ${identificador} ${nuevaGanancia}`);
  };

  const manejarCambioParametroBanda = (
    identificador: number,
    frecuenciaHz: number,
    factorQ: number,
    gananciaDb: number,
    tipo: TipoFiltro,
    activo: boolean
  ) => {
    setEstado((previo) => ({
      ...previo,
      bandas: previo.bandas.map((b) =>
        b.identificador === identificador
          ? {
              ...b,
              frecuenciaHz,
              factorQ,
              gananciaDb,
              tipo,
              activo,
              etiqueta: formatearEtiquetaFrecuencia(frecuenciaHz),
              nombreHumano: formatearEtiquetaFrecuencia(frecuenciaHz),
            }
          : b
      ),
    }));
    enviarComandoMotorRust(
      `PARAM_BANDA ${identificador} ${frecuenciaHz} ${factorQ} ${gananciaDb} ${tipo} ${
        activo ? 1 : 0
      }`
    );
  };

  const manejarAgregarBanda = () => {
    enviarComandoMotorRust('AGREGAR_BANDA 2500.0 1.41 0.0 PicoParametrico');
    setMensajeEstado('Nueva banda parametrica agregada al banco Biquad');
  };

  const manejarEliminarBanda = (idBanda: number) => {
    if (estado.bandas.length <= 1) return;
    enviarComandoMotorRust(`ELIMINAR_BANDA ${idBanda}`);
    setBandaSeleccionadaId(1);
    setMensajeEstado(`Banda #${idBanda} eliminada del banco Biquad`);
  };

  const manejarCambioTono = (nuevosCampos: Partial<EstadoEcualizador['tono']>) => {
    const t = { ...estado.tono, ...nuevosCampos };
    setEstado((previo) => ({ ...previo, tono: t }));
    enviarComandoMotorRust(
      `TONO ${t.gravesGananciaDb} ${t.agudosGananciaDb} ${t.gravesFrecuenciaHz} ${t.gravesFactorQ} ${t.agudosFrecuenciaHz} ${t.agudosFactorQ} ${
        t.tonoActivo ? 1 : 0
      }`
    );
  };

  const manejarCambioEspacio = (nuevosCampos: Partial<EstadoEcualizador['espacio']>) => {
    const esp = { ...estado.espacio, ...nuevosCampos };
    setEstado((previo) => ({ ...previo, espacio: esp }));
    enviarComandoMotorRust(
      `ESPACIO ${esp.balance} ${esp.expansionEstereo} ${esp.modoMono ? 1 : 0} ${esp.tempo}`
    );
  };

  const manejarCambioCompresor = (nuevosCampos: Partial<EstadoEcualizador['compresor']>) => {
    const comp = { ...estado.compresor, ...nuevosCampos };
    setEstado((previo) => ({
      ...previo,
      compresor: comp,
      compresorUmbralDb: comp.limiteDb,
      compresorRatio: comp.proporcion,
    }));
    enviarComandoMotorRust(
      `COMPRESOR ${comp.activo ? 1 : 0} ${comp.limiteDb} ${comp.proporcion} ${comp.ataqueMs} ${comp.lanzamientoMs} ${comp.frecuenciaSidechainHz} ${comp.gananciaMaquillajeDb}`
    );
  };

  const manejarCambioReverberacion = (
    nuevosCampos: Partial<EstadoEcualizador['reverberacion']>
  ) => {
    const rev = { ...estado.reverberacion, ...nuevosCampos };
    setEstado((previo) => ({ ...previo, reverberacion: rev }));
    enviarComandoMotorRust(
      `REVERB ${rev.activo ? 1 : 0} ${rev.tamano} ${rev.preretardoMs} ${rev.amortiguacion} ${rev.filtroTonal} ${rev.mezclado}`
    );
  };

  const manejarReinicioPlano = () => {
    enviarComandoMotorRust('REINICIAR');
    setEstado((previo) => ({
      ...previo,
      gananciaPreampDb: 0.0,
      perfilActual: 'Sonido Original (Plano)',
      bandas: previo.bandas.map((b) => ({ ...b, gananciaDb: 0.0 })),
      tono: { ...previo.tono, gravesGananciaDb: 0.0, agudosGananciaDb: 0.0 },
    }));
    setMensajeEstado('Reinicio completado: Cadena DSP en respuesta plana 0.0 dB');
  };

  const cargarPresetJblJson = () => {
    enviarComandoMotorRust(
      'PRESET_JSON JBL Tune 720BT (passive) [28 jul 2026 8_55_44\u202fp.m.].json'
    );
    setMensajeEstado('Preset JSON real cargado: JBL Tune 720BT (passive)');
  };

  const cargarAjustesPeqSettings = () => {
    enviarComandoMotorRust('PEQ_SETTINGS 11-jul-2026-9-09-38-p-m-.peq-settings');
    setMensajeEstado('Archivo .peq-settings decodificado y aplicado al motor Rust');
  };

  const cargarModeloAutoEq = (item: ResumenPresetAutoEqUi) => {
    enviarComandoMotorRust(`AUTOEQ @${item.desplazamientoBlob}`);
    setMensajeEstado(
      `Perfil AutoEQ cargado desde auto_eq.db: ${item.nombreModelo} (${item.numeroBandas} bandas)`
    );
  };

  const bandaActiva =
    estado.bandas.find((b) => b.identificador === bandaSeleccionadaId) ||
    estado.bandas[0];

  return (
    <div className="contenedor-espacial-bento">
      {/* COLUMNA IZQUIERDA: TONO, PREAMP Y ESTADO */}
      <TarjetaBento rowSpan={2} titulo="Tono & Preamplificador" indicador="Coefs.smali 64-bit">
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            gap: '16px',
            flex: 1,
            justifyContent: 'space-between',
          }}
        >
          <PerillaRotativa3D
            etiqueta="Preamp Maestro"
            valor={estado.gananciaPreampDb}
            minimo={-15}
            maximo={15}
            unidad="dB"
            descripcion="Ganancia de preamplificacion global antes de la cascada Biquad"
            alCambiarValor={(val) => {
              setEstado({ ...estado, gananciaPreampDb: val });
              enviarComandoMotorRust(`PREAMP ${val}`);
            }}
          />

          <div
            style={{
              display: 'flex',
              gap: '18px',
              width: '100%',
              justifyContent: 'center',
              borderTop: '1px solid rgba(255,255,255,0.06)',
              paddingTop: '12px',
            }}
          >
            <PerillaRotativa3D
              etiqueta={`Graves (${Math.round(estado.tono.gravesFrecuenciaHz)}Hz)`}
              valor={estado.tono.gravesGananciaDb}
              minimo={-12}
              maximo={12}
              unidad="dB"
              descripcion="Estante de Graves (Low Shelf) nativo de Poweramp"
              alCambiarValor={(val) => manejarCambioTono({ gravesGananciaDb: val })}
            />
            <PerillaRotativa3D
              etiqueta={`Agudos (${Math.round(estado.tono.agudosFrecuenciaHz / 1000)}kHz)`}
              valor={estado.tono.agudosGananciaDb}
              minimo={-12}
              maximo={12}
              unidad="dB"
              descripcion="Estante de Agudos (High Shelf) nativo de Poweramp"
              alCambiarValor={(val) => manejarCambioTono({ agudosGananciaDb: val })}
            />
          </div>

          {/* Controles finos de Frecuencia y Q para Tono Graves/Agudos */}
          <div
            style={{
              width: '100%',
              background: 'rgba(0,0,0,0.35)',
              borderRadius: '12px',
              padding: '10px',
              border: '1px solid rgba(255,255,255,0.05)',
              display: 'flex',
              flexDirection: 'column',
              gap: '6px',
              fontSize: '0.72rem',
            }}
          >
            <div style={{ display: 'flex', justifyContent: 'space-between', color: '#d1d5db' }}>
              <span>Graves Fc: {Math.round(estado.tono.gravesFrecuenciaHz)} Hz</span>
              <span>Q: {estado.tono.gravesFactorQ.toFixed(2)}</span>
            </div>
            <input
              type="range"
              min={30}
              max={250}
              step={5}
              value={estado.tono.gravesFrecuenciaHz}
              onChange={(e) =>
                manejarCambioTono({ gravesFrecuenciaHz: parseFloat(e.target.value) })
              }
              style={{ width: '100%', accentColor: '#F59E0B' }}
            />
            <div style={{ display: 'flex', justifyContent: 'space-between', color: '#d1d5db' }}>
              <span>Agudos Fc: {Math.round(estado.tono.agudosFrecuenciaHz)} Hz</span>
              <span>Q: {estado.tono.agudosFactorQ.toFixed(2)}</span>
            </div>
            <input
              type="range"
              min={4000}
              max={16000}
              step={250}
              value={estado.tono.agudosFrecuenciaHz}
              onChange={(e) =>
                manejarCambioTono({ agudosFrecuenciaHz: parseFloat(e.target.value) })
              }
              style={{ width: '100%', accentColor: '#F59E0B' }}
            />
          </div>

          <div
            style={{
              width: '100%',
              borderTop: '1px solid rgba(255,255,255,0.06)',
              paddingTop: '12px',
              display: 'flex',
              flexDirection: 'column',
              gap: '8px',
            }}
          >
            <button
              onClick={() => {
                const nuevoEstado = !encendido;
                setEncendido(nuevoEstado);
                enviarComandoMotorRust(`ACTIVO ${nuevoEstado ? 1 : 0}`);
                setMensajeEstado(
                  nuevoEstado
                    ? 'Cadena DSP Rust Activa y Procesando Audio'
                    : 'Ecualizador en Modo Bypass (Audio Original)'
                );
              }}
              className="boton-clay-ambar"
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: '8px',
                background: encendido
                  ? 'linear-gradient(135deg, var(--ambar-sol-brillante), var(--ambar-sol-calido))'
                  : '#2d2921',
                color: encendido ? '#000' : '#d1d5db',
                width: '100%',
                padding: '11px 16px',
                boxShadow: encendido ? '0px 4px 14px rgba(245, 158, 11, 0.4)' : 'none',
              }}
            >
              <Power size={17} />
              {encendido ? 'MOTOR DSP ACTIVADO' : 'BYPASS (SONIDO ORIGINAL)'}
            </button>
          </div>
        </div>
      </TarjetaBento>

      {/* PANEL SUPERIOR CENTRAL: ESPECTRO FFT + CURVA IIR + NAVEGADOR DE 4 PANTALLAS POWERAMP */}
      <TarjetaBento colSpan={1} estiloAdicional={{ padding: '14px' }}>
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            marginBottom: '8px',
            gap: '8px',
            flexWrap: 'wrap',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <Activity size={18} color="var(--ambar-sol-brillante)" />
            <span
              style={{
                fontFamily: 'var(--fuente-titulos)',
                fontSize: '0.88rem',
                color: '#fff',
                fontWeight: 700,
              }}
            >
              ESPECTRO FFT 1024 + CURVA IIR ({estado.perfilActual})
            </span>
          </div>

          {/* Selector de las 4 Pantallas Oficiales de Poweramp Equalizer */}
          <div style={{ display: 'flex', gap: '6px' }}>
            {(
              [
                { id: 'ecualizador', texto: 'Ecualizador', Icono: Sliders },
                { id: 'espacio', texto: 'Espacio/Tempo', Icono: Radio },
                { id: 'compresor', texto: 'Compresor', Icono: Gauge },
                { id: 'reverberacion', texto: 'Reverb', Icono: Waves },
              ] as const
            ).map(({ id, texto, Icono }) => (
              <button
                key={id}
                onClick={() => setVistaActiva(id)}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '5px',
                  padding: '5px 10px',
                  borderRadius: '8px',
                  fontSize: '0.73rem',
                  fontWeight: 700,
                  cursor: 'pointer',
                  fontFamily: 'var(--fuente-ui)',
                  background:
                    vistaActiva === id
                      ? 'var(--ambar-sol-brillante)'
                      : 'rgba(255,255,255,0.05)',
                  color: vistaActiva === id ? '#000' : '#e5e7eb',
                  border:
                    vistaActiva === id
                      ? '1px solid var(--ambar-sol-brillante)'
                      : '1px solid rgba(255,255,255,0.08)',
                }}
              >
                <Icono size={13} />
                {texto}
              </button>
            ))}
          </div>
        </div>

        <div style={{ flex: 1 }}>
          <VisualizadorEspectroAmbar
            gananciasBandas={estado.bandas.map((b) => b.gananciaDb)}
            barrasEspectroTiempoReal={barrasEspectroVivo}
            curvaRespuesta={curvaRespuestaIir}
          />
        </div>
      </TarjetaBento>

      {/* COLUMNA DERECHA: BUSCADOR AUTOEQ (auto_eq.db) Y PRESETS POWERAMP (.json / .peq-settings) */}
      <TarjetaBento rowSpan={2} titulo="Base AutoEQ & Presets" indicador="auto_eq.db APK">
        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', flex: 1 }}>
          <div style={{ display: 'flex', gap: '6px' }}>
            <motion.button
              whileHover={{ scale: 1.02 }}
              whileTap={{ scale: 0.98 }}
              onClick={cargarPresetJblJson}
              style={{
                flex: 1,
                background:
                  'linear-gradient(135deg, rgba(245, 158, 11, 0.25), rgba(217, 119, 6, 0.15))',
                border: '1px solid var(--ambar-sol-brillante)',
                borderRadius: '10px',
                padding: '8px 10px',
                color: '#fff',
                fontFamily: 'var(--fuente-ui)',
                fontSize: '0.74rem',
                fontWeight: 700,
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
              }}
            >
              <Headphones size={15} color="var(--ambar-sol-brillante)" />
              <span>JBL 720BT (.json)</span>
            </motion.button>

            <motion.button
              whileHover={{ scale: 1.02 }}
              whileTap={{ scale: 0.98 }}
              onClick={cargarAjustesPeqSettings}
              style={{
                flex: 1,
                background: 'rgba(255, 42, 95, 0.16)',
                border: '1px solid #FF2A5F',
                borderRadius: '10px',
                padding: '8px 10px',
                color: '#fff',
                fontFamily: 'var(--fuente-ui)',
                fontSize: '0.74rem',
                fontWeight: 700,
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
              }}
            >
              <FolderOpen size={15} color="#FF2A5F" />
              <span>.peq-settings</span>
            </motion.button>
          </div>

          {/* Buscador en tiempo real sobre los miles de audifonos de auto_eq.db */}
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              background: 'rgba(0,0,0,0.45)',
              borderRadius: '10px',
              padding: '6px 10px',
              border: '1px solid rgba(255,255,255,0.1)',
            }}
          >
            <Search size={14} color="var(--ambar-sol-brillante)" />
            <input
              type="text"
              value={consultaAutoEq}
              placeholder="Buscar audifono en auto_eq.db (ej. Sony, Sennheiser, JBL)..."
              onChange={(e) => {
                setConsultaAutoEq(e.target.value);
                buscarEnBaseDatosAutoEq(e.target.value);
              }}
              style={{
                background: 'transparent',
                border: 'none',
                outline: 'none',
                color: '#fff',
                fontFamily: 'var(--fuente-datos)',
                fontSize: '0.75rem',
                width: '100%',
              }}
            />
          </div>

          <span style={{ fontSize: '0.7rem', color: '#9ca3af', fontWeight: 600 }}>
            MODELOS EXTRAIDOS DE AUTO_EQ.DB (APK POWERAMP)
          </span>

          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: '6px',
              maxHeight: '260px',
              overflowY: 'auto',
              paddingRight: '4px',
            }}
          >
            {resultadosAutoEq.length > 0 ? (
              resultadosAutoEq.slice(0, 12).map((item) => (
                <motion.button
                  key={item.desplazamientoBlob}
                  whileHover={{ x: 3 }}
                  whileTap={{ scale: 0.98 }}
                  onClick={() => cargarModeloAutoEq(item)}
                  style={{
                    background:
                      estado.perfilActual === item.nombreModelo
                        ? 'rgba(245, 158, 11, 0.22)'
                        : 'rgba(255, 255, 255, 0.03)',
                    border: `1px solid ${
                      estado.perfilActual === item.nombreModelo
                        ? 'var(--ambar-sol-brillante)'
                        : 'rgba(255,255,255,0.06)'
                    }`,
                    borderRadius: '10px',
                    padding: '8px 10px',
                    color: '#fff',
                    fontFamily: 'var(--fuente-ui)',
                    fontSize: '0.76rem',
                    fontWeight: 600,
                    textAlign: 'left',
                    cursor: 'pointer',
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'center',
                  }}
                >
                  <span
                    style={{
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                      maxWidth: '175px',
                    }}
                  >
                    {item.nombreModelo}
                  </span>
                  <span
                    style={{
                      fontFamily: 'var(--fuente-datos)',
                      fontSize: '0.66rem',
                      color: 'var(--ambar-sol-brillante)',
                    }}
                  >
                    {item.numeroBandas}B {item.esParametrico ? 'PEQ' : 'GEQ'}
                  </span>
                </motion.button>
              ))
            ) : (
              <div style={{ fontSize: '0.74rem', color: '#9ca3af', padding: '8px 0' }}>
                Escribe una marca o modelo para consultar la base binaria PaP\x01.
              </div>
            )}
          </div>

          <div style={{ marginTop: 'auto', paddingTop: '8px' }}>
            <button
              onClick={manejarReinicioPlano}
              style={{
                width: '100%',
                padding: '9px',
                borderRadius: '10px',
                background: 'transparent',
                border: '1px dashed rgba(255,255,255,0.25)',
                color: '#e5e7eb',
                fontFamily: 'var(--fuente-ui)',
                fontSize: '0.78rem',
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: '8px',
              }}
            >
              <RotateCcw size={14} /> Reiniciar cadena DSP a 0 dB
            </button>
          </div>
        </div>
      </TarjetaBento>

      {/* PANEL CENTRAL PRINCIPAL: CAMBIA SEGUN LA PANTALLA ACTIVA DE POWERAMP */}
      {vistaActiva === 'ecualizador' && (
        <TarjetaBento
          colSpan={1}
          titulo={`Banco Parametrico (${estado.bandas.length} Bandas Activas)`}
          indicador="+/- 24 dB | RBJ 64-bit"
        >
          <div style={{ display: 'flex', flexDirection: 'column', gap: '10px', flex: 1 }}>
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                flex: 1,
                padding: '2px 0',
                overflowX: 'auto',
                gap: '4px',
              }}
            >
              {estado.bandas.map((banda) => (
                <div
                  key={banda.identificador}
                  onClick={() => setBandaSeleccionadaId(banda.identificador)}
                  style={{
                    flex: 1,
                    minWidth: '48px',
                    height: '100%',
                    borderRadius: '10px',
                    padding: '2px',
                    border:
                      bandaSeleccionadaId === banda.identificador
                        ? '1px solid rgba(245, 158, 11, 0.45)'
                        : '1px solid transparent',
                  }}
                >
                  <FaderEsqueumorfico
                    banda={banda}
                    alCambiarGanancia={manejarCambioGananciaBanda}
                  />
                </div>
              ))}
            </div>

            {/* Inspector Parametrico de la Banda Seleccionada (Frecuencia, Q, Tipo de Filtro) */}
            {bandaActiva && (
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  gap: '10px',
                  background: 'rgba(0,0,0,0.4)',
                  padding: '8px 12px',
                  borderRadius: '12px',
                  border: '1px solid rgba(255,255,255,0.06)',
                  flexWrap: 'wrap',
                  fontSize: '0.74rem',
                }}
              >
                <span
                  style={{
                    fontFamily: 'var(--fuente-datos)',
                    color: 'var(--ambar-sol-brillante)',
                    fontWeight: 700,
                  }}
                >
                  BANDA #{bandaActiva.identificador}
                </span>

                <label style={{ display: 'flex', alignItems: 'center', gap: '4px', color: '#e5e7eb' }}>
                  <span>Hz:</span>
                  <input
                    type="number"
                    min={16}
                    max={22000}
                    step={1}
                    value={Math.round(bandaActiva.frecuenciaHz)}
                    onChange={(e) =>
                      manejarCambioParametroBanda(
                        bandaActiva.identificador,
                        Number(e.target.value) || 1000,
                        bandaActiva.factorQ,
                        bandaActiva.gananciaDb,
                        bandaActiva.tipo,
                        bandaActiva.activo
                      )
                    }
                    style={{
                      width: '68px',
                      background: '#111318',
                      color: '#fff',
                      border: '1px solid rgba(255,255,255,0.15)',
                      borderRadius: '6px',
                      padding: '3px 6px',
                      fontFamily: 'var(--fuente-datos)',
                    }}
                  />
                </label>

                <label style={{ display: 'flex', alignItems: 'center', gap: '4px', color: '#e5e7eb' }}>
                  <span>Q:</span>
                  <input
                    type="number"
                    min={0.1}
                    max={20}
                    step={0.05}
                    value={bandaActiva.factorQ}
                    onChange={(e) =>
                      manejarCambioParametroBanda(
                        bandaActiva.identificador,
                        bandaActiva.frecuenciaHz,
                        Number(e.target.value) || 1.41,
                        bandaActiva.gananciaDb,
                        bandaActiva.tipo,
                        bandaActiva.activo
                      )
                    }
                    style={{
                      width: '58px',
                      background: '#111318',
                      color: '#fff',
                      border: '1px solid rgba(255,255,255,0.15)',
                      borderRadius: '6px',
                      padding: '3px 6px',
                      fontFamily: 'var(--fuente-datos)',
                    }}
                  />
                </label>

                <select
                  value={bandaActiva.tipo}
                  onChange={(e) =>
                    manejarCambioParametroBanda(
                      bandaActiva.identificador,
                      bandaActiva.frecuenciaHz,
                      bandaActiva.factorQ,
                      bandaActiva.gananciaDb,
                      e.target.value as TipoFiltro,
                      bandaActiva.activo
                    )
                  }
                  style={{
                    background: '#111318',
                    color: '#fff',
                    border: '1px solid rgba(255,255,255,0.15)',
                    borderRadius: '6px',
                    padding: '3px 8px',
                    fontFamily: 'var(--fuente-ui)',
                    fontSize: '0.73rem',
                  }}
                >
                  <option value="PicoParametrico">Pico Parametrico (PK)</option>
                  <option value="EstanteGraves">Estante Graves (LS)</option>
                  <option value="EstanteAgudos">Estante Agudos (HS)</option>
                  <option value="PasoBajo">Paso Bajo (LP)</option>
                  <option value="PasoAlto">Paso Alto (HP)</option>
                  <option value="PasoBanda">Paso Banda (BP)</option>
                </select>

                <div style={{ display: 'flex', gap: '6px' }}>
                  <button
                    onClick={manejarAgregarBanda}
                    title="Agregar banda parametrica"
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: '4px',
                      background: 'rgba(245, 158, 11, 0.2)',
                      border: '1px solid var(--ambar-sol-brillante)',
                      color: '#fff',
                      borderRadius: '6px',
                      padding: '4px 8px',
                      cursor: 'pointer',
                      fontSize: '0.72rem',
                    }}
                  >
                    <Plus size={13} /> Banda
                  </button>
                  <button
                    onClick={() => manejarEliminarBanda(bandaActiva.identificador)}
                    title="Eliminar banda seleccionada"
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: '4px',
                      background: 'rgba(255, 42, 95, 0.2)',
                      border: '1px solid #FF2A5F',
                      color: '#fff',
                      borderRadius: '6px',
                      padding: '4px 8px',
                      cursor: 'pointer',
                      fontSize: '0.72rem',
                    }}
                  >
                    <Trash2 size={13} />
                  </button>
                </div>
              </div>
            )}
          </div>
        </TarjetaBento>
      )}

      {vistaActiva === 'espacio' && (
        <TarjetaBento
          colSpan={1}
          titulo="Espacio, Imagen Estereo & Tempo"
          indicador="Matriz Mid/Side + Varispeed"
        >
          <div
            style={{
              display: 'flex',
              justifyContent: 'space-around',
              alignItems: 'center',
              flex: 1,
              flexWrap: 'wrap',
              gap: '18px',
            }}
          >
            <PerillaRotativa3D
              etiqueta="Balance L/R"
              valor={estado.espacio.balance}
              minimo={-1.0}
              maximo={1.0}
              unidad="pan"
              descripcion="Ley de paneo de potencia constante entre Canal Izquierdo (-1.0) y Derecho (+1.0)"
              alCambiarValor={(val) => manejarCambioEspacio({ balance: val })}
            />
            <PerillaRotativa3D
              etiqueta="Expansion Estereo"
              valor={Math.round(estado.espacio.expansionEstereo * 100)}
              minimo={0}
              maximo={150}
              unidad="%"
              descripcion="Apertura espacial de matriz Mid/Side de Poweramp"
              alCambiarValor={(val) => manejarCambioEspacio({ expansionEstereo: val / 100 })}
            />
            <PerillaRotativa3D
              etiqueta="Tempo / Velocidad"
              valor={estado.espacio.tempo}
              minimo={0.5}
              maximo={2.0}
              unidad="x"
              descripcion="Control de velocidad temporal en bufer circular"
              alCambiarValor={(val) => manejarCambioEspacio({ tempo: val })}
            />
            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '8px' }}>
              <span style={{ fontSize: '0.78rem', color: '#e5e7eb', fontWeight: 600 }}>
                Mezcla Monofonica
              </span>
              <button
                onClick={() => manejarCambioEspacio({ modoMono: !estado.espacio.modoMono })}
                style={{
                  padding: '10px 18px',
                  borderRadius: '10px',
                  fontWeight: 700,
                  cursor: 'pointer',
                  background: estado.espacio.modoMono
                    ? 'var(--ambar-sol-brillante)'
                    : 'rgba(255,255,255,0.06)',
                  color: estado.espacio.modoMono ? '#000' : '#fff',
                  border: '1px solid rgba(255,255,255,0.12)',
                }}
              >
                {estado.espacio.modoMono ? 'MONO ACTIVO' : 'ESTEREO NORMAL'}
              </button>
            </div>
          </div>
        </TarjetaBento>
      )}

      {vistaActiva === 'compresor' && (
        <TarjetaBento
          colSpan={1}
          titulo="Compresor Dinamico Estereo (peq_compr.xml)"
          indicador={`Reduccion: -${reduccionCompresorVivoDb.toFixed(1)} dB`}
        >
          <div
            style={{
              display: 'flex',
              justifyContent: 'space-around',
              alignItems: 'center',
              flex: 1,
              flexWrap: 'wrap',
              gap: '14px',
            }}
          >
            <PerillaRotativa3D
              etiqueta="Limite (Umbral)"
              valor={estado.compresor.limiteDb}
              minimo={-48}
              maximo={0}
              unidad="dB"
              descripcion="Nivel en dBFS a partir del cual actua la compresion de rodilla suave"
              alCambiarValor={(val) => manejarCambioCompresor({ limiteDb: val })}
            />
            <PerillaRotativa3D
              etiqueta="Proporcion"
              valor={estado.compresor.proporcion}
              minimo={1.0}
              maximo={20.0}
              unidad=":1"
              descripcion="Relacion de compresion dinamica"
              alCambiarValor={(val) => manejarCambioCompresor({ proporcion: val })}
            />
            <PerillaRotativa3D
              etiqueta="Ataque"
              valor={estado.compresor.ataqueMs}
              minimo={0.5}
              maximo={150}
              unidad="ms"
              descripcion="Tiempo de reaccion del seguidor de envolvente ante picos"
              alCambiarValor={(val) => manejarCambioCompresor({ ataqueMs: val })}
            />
            <PerillaRotativa3D
              etiqueta="Lanzamiento"
              valor={estado.compresor.lanzamientoMs}
              minimo={10}
              maximo={1000}
              unidad="ms"
              descripcion="Tiempo de recuperacion gradual de la ganancia"
              alCambiarValor={(val) => manejarCambioCompresor({ lanzamientoMs: val })}
            />
            <PerillaRotativa3D
              etiqueta="Frec. Sidechain"
              valor={estado.compresor.frecuenciaSidechainHz}
              minimo={20}
              maximo={1000}
              unidad="Hz"
              descripcion="Filtro Paso-Alto en el detector para que el sub-grave no bombee la mezcla"
              alCambiarValor={(val) => manejarCambioCompresor({ frecuenciaSidechainHz: val })}
            />
            <PerillaRotativa3D
              etiqueta="Maquillaje"
              valor={estado.compresor.gananciaMaquillajeDb}
              minimo={-12}
              maximo={18}
              unidad="dB"
              descripcion="Ganancia de compensacion posterior al compresor"
              alCambiarValor={(val) => manejarCambioCompresor({ gananciaMaquillajeDb: val })}
            />
          </div>
        </TarjetaBento>
      )}

      {vistaActiva === 'reverberacion' && (
        <TarjetaBento
          colSpan={1}
          titulo="Reverberacion Estereo Schroeder-Moorer / Freeverb"
          indicador={estado.reverberacion.activo ? 'Tanque Activo' : 'En Espera'}
        >
          <div
            style={{
              display: 'flex',
              justifyContent: 'space-around',
              alignItems: 'center',
              flex: 1,
              flexWrap: 'wrap',
              gap: '14px',
            }}
          >
            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '8px' }}>
              <span style={{ fontSize: '0.76rem', color: '#e5e7eb', fontWeight: 600 }}>
                Estado Reverb
              </span>
              <button
                onClick={() =>
                  manejarCambioReverberacion({ activo: !estado.reverberacion.activo })
                }
                style={{
                  padding: '10px 14px',
                  borderRadius: '10px',
                  fontWeight: 700,
                  cursor: 'pointer',
                  background: estado.reverberacion.activo
                    ? 'var(--ambar-sol-brillante)'
                    : 'rgba(255,255,255,0.06)',
                  color: estado.reverberacion.activo ? '#000' : '#fff',
                  border: '1px solid rgba(255,255,255,0.12)',
                }}
              >
                {estado.reverberacion.activo ? 'ACTIVADA' : 'DESACTIVADA'}
              </button>
            </div>

            <PerillaRotativa3D
              etiqueta="Tamano de Sala"
              valor={Math.round(estado.reverberacion.tamano * 100)}
              minimo={0}
              maximo={100}
              unidad="%"
              descripcion="Dimension acustica de los 8 filtros peine por canal"
              alCambiarValor={(val) =>
                manejarCambioReverberacion({ tamano: val / 100, activo: true })
              }
            />
            <PerillaRotativa3D
              etiqueta="Preretardo"
              valor={estado.reverberacion.preretardoMs}
              minimo={0}
              maximo={150}
              unidad="ms"
              descripcion="Tiempo antes de la primera reflexion de la sala"
              alCambiarValor={(val) =>
                manejarCambioReverberacion({ preretardoMs: val, activo: true })
              }
            />
            <PerillaRotativa3D
              etiqueta="Amortiguacion"
              valor={Math.round(estado.reverberacion.amortiguacion * 100)}
              minimo={0}
              maximo={100}
              unidad="%"
              descripcion="Absorcion de altas frecuencias en las paredes acusticas"
              alCambiarValor={(val) =>
                manejarCambioReverberacion({ amortiguacion: val / 100, activo: true })
              }
            />
            <PerillaRotativa3D
              etiqueta="Filtro Tonal"
              valor={Math.round(estado.reverberacion.filtroTonal * 100)}
              minimo={0}
              maximo={100}
              unidad="%"
              descripcion="Color de la cola reverberante (0% oscuro, 50% neutro, 100% brillante)"
              alCambiarValor={(val) =>
                manejarCambioReverberacion({ filtroTonal: val / 100, activo: true })
              }
            />
            <PerillaRotativa3D
              etiqueta="Mezclado Dry/Wet"
              valor={Math.round(estado.reverberacion.mezclado * 100)}
              minimo={0}
              maximo={100}
              unidad="%"
              descripcion="Proporcion de senal reverberante mezclada en la salida"
              alCambiarValor={(val) =>
                manejarCambioReverberacion({ mezclado: val / 100, activo: true })
              }
            />
          </div>
        </TarjetaBento>
      )}

      {/* FILA INFERIOR: TELEMETRIA DEL COMPRESOR/LIMITADOR Y ESTADO DEL MOTOR RUST */}
      <TarjetaBento
        colSpan={2}
        titulo="Telemetria Dinamica & Limitador Maestro"
        indicador="Precision 64-bit Flotante"
      >
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-around',
            alignItems: 'center',
            flex: 1,
            gap: '16px',
            flexWrap: 'wrap',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
            <ShieldCheck size={28} color="var(--ambar-sol-brillante)" />
            <div>
              <div
                style={{
                  fontFamily: 'var(--fuente-ui)',
                  fontWeight: 700,
                  fontSize: '0.9rem',
                  color: '#fff',
                }}
              >
                Limitador Lookahead & Compresor Sidechain Activos
              </div>
              <div style={{ fontSize: '0.76rem', color: '#d1d5db', maxWidth: '340px' }}>
                Proteccion contra recorte digital a -0.1 dBFS con interpolacion suave de coeficientes
                sin ruido de cremallera.
              </div>
            </div>
          </div>

          <div
            style={{
              display: 'flex',
              gap: '22px',
              background: 'rgba(0,0,0,0.35)',
              padding: '10px 18px',
              borderRadius: '14px',
              border: '1px solid rgba(255,255,255,0.06)',
            }}
          >
            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center' }}>
              <span style={{ fontSize: '0.68rem', color: '#9ca3af', fontWeight: 600 }}>
                UMBRAL COMPRESOR
              </span>
              <span
                style={{
                  fontFamily: 'var(--fuente-datos)',
                  color: '#fff',
                  fontWeight: 700,
                  fontSize: '0.88rem',
                }}
              >
                {estado.compresor.limiteDb.toFixed(1)} dB ({estado.compresor.proporcion.toFixed(1)}:1)
              </span>
            </div>
            <div style={{ width: '1px', background: 'rgba(255,255,255,0.1)' }} />
            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center' }}>
              <span style={{ fontSize: '0.68rem', color: '#9ca3af', fontWeight: 600 }}>
                REDUCCION EN VIVO
              </span>
              <span
                style={{
                  fontFamily: 'var(--fuente-datos)',
                  color: 'var(--ambar-sol-brillante)',
                  fontWeight: 700,
                  fontSize: '0.88rem',
                }}
              >
                -{reduccionCompresorVivoDb.toFixed(2)} dB
              </span>
            </div>
            <div style={{ width: '1px', background: 'rgba(255,255,255,0.1)' }} />
            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center' }}>
              <span style={{ fontSize: '0.68rem', color: '#9ca3af', fontWeight: 600 }}>
                PICO DE SALIDA
              </span>
              <span
                style={{
                  fontFamily: 'var(--fuente-datos)',
                  color: '#10b981',
                  fontWeight: 700,
                  fontSize: '0.88rem',
                }}
              >
                {picoSalidaVivoDb.toFixed(1)} dBFS
              </span>
            </div>
          </div>
        </div>
      </TarjetaBento>

      <TarjetaBento
        colSpan={1}
        estiloAdicional={{ justifyContent: 'center', alignItems: 'center', textAlign: 'center' }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            color: motorRustConectado ? 'var(--ambar-sol-brillante)' : '#fbbf24',
            fontWeight: 700,
            fontSize: '0.84rem',
          }}
        >
          <Cpu size={18} />
          {motorRustConectado
            ? 'MOTOR RUST CONECTADO (127.0.0.1:18765)'
            : 'MODO LOCAL AUTONOMO + APO'}
        </div>
        <div style={{ fontSize: '0.74rem', color: '#d1d5db', marginTop: '6px', fontWeight: 500 }}>
          {mensajeEstado}
        </div>
        <div
          style={{
            fontFamily: 'var(--fuente-datos)',
            fontSize: '0.68rem',
            color: '#9ca3af',
            marginTop: '4px',
          }}
        >
          Fs: {estado.frecuenciaMuestreo} Hz | Bandas: {estado.bandas.length} | Cero Asignaciones
        </div>
      </TarjetaBento>
    </div>
  );
};

export default App;
