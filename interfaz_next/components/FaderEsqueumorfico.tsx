'use client';

import React, { useRef, useEffect } from 'react';
import { gsap } from 'gsap';
import { BandaAudioUi } from '@/lib/tipos_audio';

interface PropsFader {
  banda: BandaAudioUi;
  alCambiarGanancia: (id: number, nuevaGanancia: number) => void;
}

export const FaderEsqueumorfico: React.FC<PropsFader> = ({ banda, alCambiarGanancia }) => {
  const capuchonRef = useRef<HTMLDivElement>(null);
  const canalRef = useRef<HTMLDivElement>(null);

  // Mapear ganancia (-24 dB a +24 dB) a posicion Y porcentual del canal analogico
  const porcentajeY = ((24 - banda.gananciaDb) / 48) * 100;

  useEffect(() => {
    if (capuchonRef.current) {
      gsap.to(capuchonRef.current, {
        top: `${Math.max(4, Math.min(88, porcentajeY))}%`,
        duration: 0.22,
        ease: 'power2.out',
      });
    }
  }, [porcentajeY]);

  const manejarCambioDeslizador = (evento: React.MouseEvent | React.TouchEvent) => {
    if (!canalRef.current) return;
    const rectangulo = canalRef.current.getBoundingClientRect();
    const clienteY = 'touches' in evento ? evento.touches[0].clientY : evento.clientY;
    const posicionRelativaY = Math.max(0, Math.min(rectangulo.height, clienteY - rectangulo.top));

    // Convertir de pixeles a rango dB de Poweramp (+24 dB arriba a -24 dB abajo)
    const proporcion = 1.0 - posicionRelativaY / rectangulo.height;
    const gananciaCalculada = Math.round((proporcion * 48.0 - 24.0) * 10) / 10;

    alCambiarGanancia(banda.identificador, gananciaCalculada);
  };

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        height: '100%',
        gap: '6px',
        flex: 1,
      }}
      title={
        banda.descripcion
          ? `${banda.nombreHumano} (${banda.etiqueta}): ${banda.descripcion}`
          : banda.etiqueta
      }
    >
      {/* Indicador Numerico de Ganancia */}
      <span
        style={{
          fontFamily: 'var(--fuente-datos)',
          fontSize: '0.74rem',
          fontWeight: 700,
          color:
            banda.gananciaDb > 0
              ? 'var(--ambar-sol-brillante)'
              : banda.gananciaDb < 0
              ? '#fb7185'
              : '#ffffff',
          background: 'rgba(0,0,0,0.5)',
          padding: '2px 6px',
          borderRadius: '6px',
          border: '1px solid rgba(255,255,255,0.06)',
          minWidth: '46px',
          textAlign: 'center',
        }}
      >
        {banda.gananciaDb > 0 ? `+${banda.gananciaDb.toFixed(1)}` : banda.gananciaDb.toFixed(1)} dB
      </span>

      {/* Canal Analogico con Esqueumorfismo Tactil estilo Poweramp */}
      <div
        ref={canalRef}
        className="canal-fader-analogo"
        style={{ margin: '4px 0' }}
        onMouseDown={(e) => {
          manejarCambioDeslizador(e);
          const mover = (ev: MouseEvent) => manejarCambioDeslizador(ev as unknown as React.MouseEvent);
          const soltar = () => {
            window.removeEventListener('mousemove', mover);
            window.removeEventListener('mouseup', soltar);
          };
          window.addEventListener('mousemove', mover);
          window.addEventListener('mouseup', soltar);
        }}
        onTouchStart={(e) => {
          manejarCambioDeslizador(e);
          const mover = (ev: TouchEvent) => manejarCambioDeslizador(ev as unknown as React.TouchEvent);
          const soltar = () => {
            window.removeEventListener('touchmove', mover);
            window.removeEventListener('touchend', soltar);
          };
          window.addEventListener('touchmove', mover);
          window.addEventListener('touchend', soltar);
        }}
      >
        {/* Capuchon Metalico Mate Esqueumorfico con Linea Ambar Sunlight */}
        <div
          ref={capuchonRef}
          className="capuchon-fader-esqueumorfico"
          style={{ position: 'absolute', transform: 'translateY(-50%)' }}
        >
          <div className="linea-guia-ambar" />
          <div style={{ width: '22px', height: '1px', background: 'rgba(255,255,255,0.18)' }} />
          <div style={{ width: '22px', height: '1px', background: 'rgba(255,255,255,0.06)' }} />
        </div>
      </div>

      {/* Nombre Humano Principal */}
      <span
        style={{
          fontFamily: 'var(--fuente-ui)',
          fontSize: '0.74rem',
          color: '#ffffff',
          fontWeight: 700,
          whiteSpace: 'nowrap',
        }}
      >
        {banda.nombreHumano}
      </span>

      {/* Frecuencia Hz / kHz */}
      <span
        style={{
          fontFamily: 'var(--fuente-datos)',
          fontSize: '0.68rem',
          color: 'var(--ambar-sol-brillante)',
          fontWeight: 600,
          opacity: 0.9,
        }}
      >
        {banda.etiqueta}
      </span>
    </div>
  );
};
