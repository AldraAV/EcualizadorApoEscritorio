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
      initial={{ opacity: 0, y: 15 }}
      animate={{ opacity: 1, y: 0 }}
      whileHover={{
        z: 12,
        scale: 1.006,
        transition: { type: 'spring', stiffness: 350, damping: 25 },
      }}
      transition={{ duration: 0.4, ease: [0.2, 0.8, 0.2, 1] }}
    >
      {(titulo || indicador) && (
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
          {titulo && <h3 className="titulo-espacial">{titulo}</h3>}
          {indicador && <span className="indicador-tecnico-hz">{indicador}</span>}
        </div>
      )}
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
        {children}
      </div>
    </motion.div>
  );
};
