'use client';

import React, { useRef, useEffect } from 'react';
import { gsap } from 'gsap';

interface PropsPerilla {
  etiqueta: string;
  valor: number;
  minimo: number;
  maximo: number;
  unidad: string;
  descripcion?: string;
  alCambiarValor: (nuevoValor: number) => void;
}

export const PerillaRotativa3D: React.FC<PropsPerilla> = ({
  etiqueta,
  valor,
  minimo,
  maximo,
  unidad,
  descripcion,
  alCambiarValor,
}) => {
  const perillaRef = useRef<HTMLDivElement>(null);

  // Mapear valor al angulo de rotacion analogica (-135 grados a +135 grados)
  const rango = maximo - minimo;
  const proporcion = rango > 0 ? (valor - minimo) / rango : 0.5;
  const rotacionGrados = -135 + proporcion * 270;

  useEffect(() => {
    if (perillaRef.current) {
      gsap.to(perillaRef.current, {
        rotation: rotacionGrados,
        duration: 0.18,
        ease: 'power1.out',
      });
    }
  }, [rotacionGrados]);

  const manejarArrastre = (evento: React.MouseEvent) => {
    const yInicial = evento.clientY;
    const valorInicial = valor;

    const mover = (e: MouseEvent) => {
      const deltaY = yInicial - e.clientY;
      const cambioProporcional = deltaY / 140;
      const nuevoValor = Math.max(minimo, Math.min(maximo, valorInicial + cambioProporcional * rango));
      alCambiarValor(Math.round(nuevoValor * 100) / 100);
    };

    const soltar = () => {
      window.removeEventListener('mousemove', mover);
      window.removeEventListener('mouseup', soltar);
    };

    window.addEventListener('mousemove', mover);
    window.addEventListener('mouseup', soltar);
  };

  return (
    <div
      style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '8px' }}
      title={descripcion || etiqueta}
    >
      <span style={{ fontFamily: 'var(--fuente-ui)', fontSize: '0.80rem', color: '#e5e7eb', fontWeight: 600 }}>
        {etiqueta}
      </span>

      {/* Chasis de la Perilla (Claymorphism + Skeuomorphism) */}
      <div
        style={{
          width: '60px',
          height: '60px',
          borderRadius: '50%',
          background: 'linear-gradient(145deg, #1f1b15, #0f0d0a)',
          boxShadow: 'var(--perilla-analoga-relieve)',
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          cursor: 'ns-resize',
          position: 'relative',
          userSelect: 'none',
        }}
        onMouseDown={manejarArrastre}
      >
        <div
          ref={perillaRef}
          style={{
            width: '100%',
            height: '100%',
            position: 'absolute',
            borderRadius: '50%',
            display: 'flex',
            justifyContent: 'center',
          }}
        >
          {/* Indicador de perilla ambar */}
          <div
            style={{
              width: '4px',
              height: '13px',
              background: 'var(--ambar-sol-brillante)',
              borderRadius: '2px',
              marginTop: '4px',
              boxShadow: '0 0 6px rgba(245, 158, 11, 0.7)',
            }}
          />
        </div>
      </div>

      {/* Lectura numerica */}
      <span
        style={{
          fontFamily: 'var(--fuente-datos)',
          fontSize: '0.74rem',
          color: '#ffffff',
          fontWeight: 700,
          background: 'rgba(0,0,0,0.4)',
          padding: '2px 6px',
          borderRadius: '4px',
          border: '1px solid rgba(255,255,255,0.05)',
        }}
      >
        {valor > 0 && unidad === 'dB' ? `+${valor}` : valor} {unidad}
      </span>
    </div>
  );
};
