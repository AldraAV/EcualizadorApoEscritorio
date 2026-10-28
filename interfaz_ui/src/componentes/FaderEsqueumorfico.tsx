import React, { useRef, useEffect } from 'react';
import { gsap } from 'gsap';
import { BandaAudio } from '../tipos/audio';

interface PropsFader {
  banda: BandaAudio;
  alCambiarGanancia: (id: number, nuevaGanancia: number) => void;
}

export const FaderEsqueumorfico: React.FC<PropsFader> = ({ banda, alCambiarGanancia }) => {
  const capuchonRef = useRef<HTMLDivElement>(null);
  const canalRef = useRef<HTMLDivElement>(null);

  // Mapear ganancia (-24 dB a +24 dB) a posición Y porcentual del canal analógico
  const porcentajeY = ((24 - banda.gananciaDb) / 48) * 100;

  useEffect(() => {
    if (capuchonRef.current) {
      gsap.to(capuchonRef.current, {
        top: `${Math.max(4, Math.min(88, porcentajeY))}%`,
        duration: 0.25,
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
      style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', height: '100%', gap: '6px', flex: 1 }}
      title={banda.descripcion ? `${banda.nombreHumano} (${banda.etiqueta}): ${banda.descripcion}` : banda.etiqueta}
    >
      {/* Indicador Numérico de Ganancia */}
      <span style={{ 
        fontFamily: 'var(--fuente-datos)', 
        fontSize: '0.78rem', 
        fontWeight: 700, 
        color: banda.gananciaDb > 0 ? 'var(--ambar-sol-brillante)' : banda.gananciaDb < 0 ? '#fb7185' : '#ffffff',
        background: 'rgba(0,0,0,0.4)',
        padding: '2px 6px',
        borderRadius: '6px',
        border: '1px solid rgba(255,255,255,0.05)'
      }}>
        {banda.gananciaDb > 0 ? `+${banda.gananciaDb}` : banda.gananciaDb} dB
      </span>

      {/* Canal Analógico con Esqueumorfismo Táctil estilo Poweramp */}
      <div 
        ref={canalRef}
        className="canal-fader-analogo"
        style={{ margin: '4px 0' }}
        onMouseDown={(e) => {
          manejarCambioDeslizador(e);
          const mover = (ev: MouseEvent) => manejarCambioDeslizador(ev as any);
          const soltar = () => {
            window.removeEventListener('mousemove', mover);
            window.removeEventListener('mouseup', soltar);
          };
          window.addEventListener('mousemove', mover);
          window.addEventListener('mouseup', soltar);
        }}
        onTouchStart={(e) => {
          manejarCambioDeslizador(e);
          const mover = (ev: TouchEvent) => manejarCambioDeslizador(ev as any);
          const soltar = () => {
            window.removeEventListener('touchmove', mover);
            window.removeEventListener('touchend', soltar);
          };
          window.addEventListener('touchmove', mover);
          window.addEventListener('touchend', soltar);
        }}
      >
        {/* Capuchón Metálico Mate Esqueumórfico con Línea Ámbar Sunlight */}
        <div 
          ref={capuchonRef}
          className="capuchon-fader-esqueumorfico"
          style={{ position: 'absolute', transform: 'translateY(-50%)' }}
        >
          <div className="linea-guia-ambar" />
          <div style={{ width: '22px', height: '1px', background: 'rgba(255,255,255,0.15)' }} />
          <div style={{ width: '22px', height: '1px', background: 'rgba(255,255,255,0.05)' }} />
        </div>
      </div>

      {/* Nombre Humano Principal (Lo que entiende el usuario) */}
      <span style={{ 
        fontFamily: 'var(--fuente-ui)', 
        fontSize: '0.78rem', 
        color: 'var(--ambar-sol-brillante)', 
        fontWeight: 700,
        textAlign: 'center',
        lineHeight: 1.1,
        height: '28px',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center'
      }}>
        {banda.nombreHumano || banda.etiqueta}
      </span>

      {/* Frecuencia Técnica Secundaria */}
      <span style={{ 
        fontFamily: 'var(--fuente-datos)', 
        fontSize: '0.68rem', 
        color: '#9ca3af', 
        fontWeight: 500 
      }}>
        {banda.etiqueta}
      </span>
    </div>
  );
};
