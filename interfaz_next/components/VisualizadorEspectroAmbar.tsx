'use client';

import React, { useEffect, useRef } from 'react';

interface PropsVisualizador {
  gananciasBandas: number[];
  barrasEspectroTiempoReal?: number[];
  curvaRespuesta?: number[];
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
    const numeroBarras = 64;
    const alturasSuavizadas = new Array(numeroBarras).fill(0);

    // Formula de suavizado de milk_main.lua de Poweramp
    const suavizado = 0.72;
    const mezcla1 = Math.pow(suavizado * 0.97, 0.5);
    const mezcla2 = 1.0 - mezcla1;

    const animar = () => {
      const ancho = lienzo.width;
      const alto = lienzo.height;
      contexto.clearRect(0, 0, ancho, alto);

      // 1. Rejilla acustica de referencia (+12 dB, 0 dB, -12 dB)
      contexto.strokeStyle = 'rgba(255, 255, 255, 0.05)';
      contexto.lineWidth = 1;
      [0.25, 0.5, 0.75].forEach((proporcionY) => {
        const yLinea = alto * proporcionY;
        contexto.beginPath();
        contexto.moveTo(0, yLinea);
        contexto.lineTo(ancho, yLinea);
        contexto.stroke();
      });

      // 2. Barras del analizador de espectro FFT
      const anchuraBarra = Math.max(2, ancho / numeroBarras - 2.5);

      for (let i = 0; i < numeroBarras; i++) {
        let alturaObjetivo = 8;

        if (barrasEspectroTiempoReal && barrasEspectroTiempoReal.length > 0) {
          const idxReal = Math.min(
            barrasEspectroTiempoReal.length - 1,
            Math.floor((i / numeroBarras) * barrasEspectroTiempoReal.length)
          );
          const magnitudNorm = barrasEspectroTiempoReal[idxReal] || 0;
          alturaObjetivo = Math.max(4, magnitudNorm * (alto - 12));
        } else {
          const indiceBanda = Math.floor((i / numeroBarras) * Math.max(1, gananciasBandas.length));
          const gananciaBase = gananciasBandas[indiceBanda] || 0;
          const ondaViva = Math.sin(Date.now() / 200 + i * 0.4) * 8;
          alturaObjetivo = Math.max(6, ((gananciaBase + 24) / 48) * (alto * 0.8) + ondaViva);
        }

        alturasSuavizadas[i] = alturaObjetivo * mezcla2 + alturasSuavizadas[i] * mezcla1;

        const gradienteBarra = contexto.createLinearGradient(0, alto, 0, 0);
        gradienteBarra.addColorStop(0, 'rgba(217, 119, 6, 0.30)');
        gradienteBarra.addColorStop(0.65, 'rgba(245, 158, 11, 0.65)');
        gradienteBarra.addColorStop(1, 'rgba(251, 191, 36, 0.90)');

        contexto.fillStyle = gradienteBarra;
        const x = i * (anchuraBarra + 2.5) + 2;
        const h = Math.min(alto - 4, Math.max(3, alturasSuavizadas[i]));
        const y = alto - h;

        contexto.beginPath();
        contexto.roundRect(x, y, anchuraBarra, h, [2, 2, 0, 0]);
        contexto.fill();
      }

      // 3. Curva continua de respuesta en frecuencia Biquad (Coefs.smali)
      contexto.save();
      contexto.beginPath();

      if (curvaRespuesta && curvaRespuesta.length > 1) {
        curvaRespuesta.forEach((db, idx) => {
          const x = (idx / (curvaRespuesta.length - 1)) * ancho;
          const dbAcotado = Math.max(-24, Math.min(24, db));
          const y = alto * 0.5 - (dbAcotado / 24) * (alto * 0.44);
          if (idx === 0) {
            contexto.moveTo(x, y);
          } else {
            contexto.lineTo(x, y);
          }
        });
      } else {
        // Trazado de interpolacion suave usando las ganancias de las bandas
        const pasoX = ancho / Math.max(1, gananciasBandas.length - 1);
        gananciasBandas.forEach((db, idx) => {
          const x = idx * pasoX;
          const y = alto * 0.5 - (db / 24) * (alto * 0.44);
          if (idx === 0) {
            contexto.moveTo(x, y);
          } else {
            const xAnt = (idx - 1) * pasoX;
            const yAnt = alto * 0.5 - (gananciasBandas[idx - 1] / 24) * (alto * 0.44);
            const xControl1 = xAnt + pasoX * 0.5;
            const xControl2 = xAnt + pasoX * 0.5;
            contexto.bezierCurveTo(xControl1, yAnt, xControl2, y, x, y);
          }
        });
      }

      contexto.strokeStyle = 'rgba(255, 255, 255, 0.95)';
      contexto.lineWidth = 2.5;
      contexto.shadowColor = 'rgba(245, 158, 11, 0.8)';
      contexto.shadowBlur = 10;
      contexto.stroke();
      contexto.restore();

      idAnimacion = requestAnimationFrame(animar);
    };

    animar();

    return () => {
      cancelAnimationFrame(idAnimacion);
    };
  }, [gananciasBandas, barrasEspectroTiempoReal, curvaRespuesta]);

  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        position: 'relative',
        background: 'radial-gradient(ellipse at 50% 120%, rgba(245, 158, 11, 0.08) 0%, rgba(7, 9, 14, 0.95) 75%)',
        borderRadius: '14px',
        overflow: 'hidden',
        border: '1px solid rgba(255, 255, 255, 0.06)',
      }}
    >
      <canvas
        ref={lienzoRef}
        width={900}
        height={220}
        style={{ width: '100%', height: '100%', display: 'block' }}
      />
      <div
        style={{
          position: 'absolute',
          top: '8px',
          left: '12px',
          display: 'flex',
          gap: '8px',
          alignItems: 'center',
          pointerEvents: 'none',
        }}
      >
        <span
          style={{
            fontFamily: 'var(--fuente-datos)',
            fontSize: '0.68rem',
            color: 'var(--ambar-sol-brillante)',
            fontWeight: 700,
            background: 'rgba(0,0,0,0.6)',
            padding: '2px 8px',
            borderRadius: '6px',
            border: '1px solid rgba(245, 158, 11, 0.3)',
          }}
        >
          FFT 64 BARRAS + CURVA BIQUAD
        </span>
      </div>
    </div>
  );
};
