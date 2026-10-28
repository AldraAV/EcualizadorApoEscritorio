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

  // Mapear valor al ángulo de rotación analógica (-135 grados a +135 grados)
  const proporcion = (valor - minimo) / (maximo - minimo);
  const rotacionGrados = -135 + proporcion * 270;

  useEffect(() => {
    if (perillaRef.current) {
      gsap.to(perillaRef.current, {
        rotation: rotacionGrados,
        duration: 0.2,
        ease: 'power1.out',
      });
    }
  }, [rotacionGrados]);

  const manejarArrastre = (evento: React.MouseEvent) => {
    const yInicial = evento.clientY;
    const valorInicial = valor;

    const mover = (e: MouseEvent) => {
      const deltaY = yInicial - e.clientY;
      const cambioProporcional = deltaY / 120;
      const nuevoValor = Math.max(minimo, Math.min(maximo, valorInicial + cambioProporcional * (maximo - minimo)));
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
      style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '10px' }}
      title={descripcion || etiqueta}
    >
      <span style={{ fontFamily: 'var(--fuente-ui)', fontSize: '0.82rem', color: '#e5e7eb', fontWeight: 600 }}>
        {etiqueta}
      </span>

      {/* Chasis de la Perilla (Claymorphism + Skeuomorphism) */}
      <div 
        style={{
          width: '64px',
          height: '64px',
          borderRadius: '50%',
          background: 'linear-gradient(145deg, #1f1b15, #0f0d0a)',
          boxShadow: 'var(--perilla-analoga-relieve)',
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          cursor: 'ns-resize',
          position: 'relative'
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
            justifyContent: 'center'
          }}
        >
          {/* Indicador de perilla ámbar */}
          <div style={{
            width: '4px',
            height: '14px',
            background: 'var(--ambar-sol-brillante)',
            borderRadius: '2px',
            marginTop: '6px',
            boxShadow: '0 0 6px var(--ambar-sol-brillante)'
          }} />
        </div>
        <div style={{
          width: '32px',
          height: '32px',
          borderRadius: '50%',
          background: '#161410',
          boxShadow: 'inset 2px 2px 4px rgba(0,0,0,0.8)'
        }} />
      </div>

      <span style={{ fontFamily: 'var(--fuente-datos)', fontSize: '0.82rem', color: 'var(--ambar-sol-brillante)', fontWeight: 600 }}>
        {valor} {unidad}
      </span>
    </div>
  );
};
