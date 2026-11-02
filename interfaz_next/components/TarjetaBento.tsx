'use client';

import React from 'react';
import { motion } from 'framer-motion';

interface PropsTarjetaBento {
  titulo?: string;
  indicador?: string;
  colSpan?: number;
  rowSpan?: number;
  children: React.ReactNode;
  estiloAdicional?: React.CSSProperties;
}

export const TarjetaBento: React.FC<PropsTarjetaBento> = ({
  titulo,
  indicador,
  colSpan = 1,
  rowSpan = 1,
  children,
  estiloAdicional,
}) => {
  return (
    <motion.div
      className="tarjeta-bento-clay"
      style={{
        gridColumn: `span ${colSpan}`,
        gridRow: `span ${rowSpan}`,
        ...estiloAdicional,
      }}
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.35, ease: 'easeOut' }}
    >
      {(titulo || indicador) && (
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            marginBottom: '12px',
          }}
        >
          {titulo && <h3 className="titulo-espacial">{titulo}</h3>}
          {indicador && <span className="indicador-tecnico-hz">{indicador}</span>}
        </div>
      )}
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>{children}</div>
    </motion.div>
  );
};
