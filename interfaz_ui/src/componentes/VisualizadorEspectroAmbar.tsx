import React, { useEffect, useRef } from 'react';
import { PuntoCurvaRespuesta } from '../tipos/audio';

interface PropsVisualizador {
  gananciasBandas: number[];
  barrasEspectroTiempoReal?: number[];
  curvaRespuesta?: PuntoCurvaRespuesta[];
}

export const VisualizadorEspectroAmbar: React.FC<PropsVisualizador> = ({
  gananciasBandas,
  barrasEspectroTiempoReal,
  curvaRespuesta,
}) => {
  const lienzoRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const lienzo = lienzoRef.current;
    if (!lienzo) return;
    const contexto = lienzo.getContext('2d');
    if (!contexto) return;

    let idAnimacion: number;
    const numeroBarras = 48;
    const alturasSuavizadas = new Array(numeroBarras).fill(0);

    // Formula de suavizado extraida de assets/milk/lua/milk_main.lua de Poweramp
    const suavizado = 0.72;
    const mezcla1 = Math.pow(suavizado * 0.97, 0.5);
    const mezcla2 = 1.0 - mezcla1;

    const animar = () => {
      const ancho = lienzo.width;
      const alto = lienzo.height;
      contexto.clearRect(0, 0, ancho, alto);

      // 1. Rejilla acustica de referencia (+12 dB, 0 dB, -12 dB)
      contexto.strokeStyle = 'rgba(255, 255, 255, 0.06)';
      contexto.lineWidth = 1;
      [0.2, 0.5, 0.8].forEach((proporcionY) => {
        const yLinea = alto * proporcionY;
        contexto.beginPath();
        contexto.moveTo(0, yLinea);
        contexto.lineTo(ancho, yLinea);
        contexto.stroke();
      });

      // 2. Barras del analizador de espectro FFT en tiempo real
      const anchuraBarra = ancho / numeroBarras - 3;

      for (let i = 0; i < numeroBarras; i++) {
        let alturaObjetivo = 12;

        if (barrasEspectroTiempoReal && barrasEspectroTiempoReal.length > 0) {
          const idxReal = Math.min(
            barrasEspectroTiempoReal.length - 1,
            Math.floor((i / numeroBarras) * barrasEspectroTiempoReal.length)
          );
          const magnitudNorm = barrasEspectroTiempoReal[idxReal] || 0;
          alturaObjetivo = Math.max(6, magnitudNorm * (alto - 14));
        } else {
          const indiceBanda = Math.floor((i / numeroBarras) * Math.max(1, gananciasBandas.length));
          const gananciaBase = gananciasBandas[indiceBanda] || 0;
          const ondaViva = Math.sin(Date.now() / 180 + i * 0.45) * 10;
          alturaObjetivo = Math.max(8, ((gananciaBase + 24) / 48) * (alto * 0.82) + ondaViva);
        }

        alturasSuavizadas[i] =
          alturaObjetivo * mezcla2 + alturasSuavizadas[i] * mezcla1;

        const gradienteBarra = contexto.createLinearGradient(0, alto, 0, 0);
        gradienteBarra.addColorStop(0, 'rgba(217, 119, 6, 0.35)');
        gradienteBarra.addColorStop(0.65, 'rgba(245, 158, 11, 0.65)');
        gradienteBarra.addColorStop(1, 'rgba(251, 191, 36, 0.90)');

        contexto.fillStyle = gradienteBarra;
        const x = i * (anchuraBarra + 3) + 2;
        const h = Math.min(alto - 4, Math.max(4, alturasSuavizadas[i]));
        const y = alto - h;

        contexto.beginPath();
        contexto.roundRect(x, y, anchuraBarra, h, [3, 3, 0, 0]);
        contexto.fill();
      }

      // 3. Curva continua de respuesta en frecuencia IIR (Coefs.smali de Poweramp)
      contexto.save();
      contexto.beginPath();

      if (curvaRespuesta && curvaRespuesta.length > 1) {
        curvaRespuesta.forEach((punto, idx) => {
          const x = (idx / (curvaRespuesta.length - 1)) * ancho;
          const dbAcotado = Math.max(-24, Math.min(24, punto.gananciaDb));
          const y = alto * 0.5 - (dbAcotado / 24) * (alto * 0.42);
          if (idx === 0) {
            contexto.moveTo(x, y);
          } else {
            contexto.lineTo(x, y);
          }
        });
      } else if (gananciasBandas.length > 0) {
        gananciasBandas.forEach((db, idx) => {
          const x =
            gananciasBandas.length === 1
              ? ancho * 0.5
              : (idx / (gananciasBandas.length - 1)) * ancho;
          const dbAcotado = Math.max(-24, Math.min(24, db));
          const y = alto * 0.5 - (dbAcotado / 24) * (alto * 0.42);
          if (idx === 0) {
            contexto.moveTo(x, y);
          } else {
            contexto.lineTo(x, y);
          }
        });
      }

      contexto.strokeStyle = '#FF2A5F';
      contexto.lineWidth = 2.4;
      contexto.shadowColor = 'rgba(255, 42, 95, 0.55)';
      contexto.shadowBlur = 10;
      contexto.stroke();
      contexto.restore();

      idAnimacion = requestAnimationFrame(animar);
    };

    animar();

    return () => cancelAnimationFrame(idAnimacion);
  }, [gananciasBandas, barrasEspectroTiempoReal, curvaRespuesta]);

  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        position: 'relative',
        overflow: 'hidden',
        borderRadius: '16px',
        background: 'var(--fondo-espacial-0)',
        border: '1px solid rgba(245, 158, 11, 0.18)',
        padding: '10px',
      }}
    >
      <canvas
        ref={lienzoRef}
        width={680}
        height={135}
        style={{ width: '100%', height: '100%', display: 'block' }}
      />
    </div>
  );
};
