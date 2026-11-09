# 🎛️ Plan de Arquitectura: Ecualizador Soberano en Linux (PipeWire + Rust Iced)

> **ESTATUS: ESPECIFICACIÓN TÉCNICA Y ARQUITECTURA DE MIGRACIÓN A LINUX**  
> Proyecto: `EcualizadorApoEscritorio` (`motor_rust`)  
> Entorno Objetivo: Linux (X11 / Wayland con PipeWire) y compatibilidad Windows.  
> Estética: **Obsidian Rack Acoustic** (Aurora Slate, Ámbar Sunlight, JetBrains Mono).  
> Soberanía Lingüística: 100% en español y cero emojis en código fuente.

---

## 🏛️ 1. Objetivo General

Dotar al ecualizador de escritorio en Rust (`motor_dsp_poweramp`) de:
1. **Soporte Nativo para PipeWire en Linux:**
   - Detección y enumeración en tiempo real de reproductores y aplicaciones emitiendo audio (`Spotify`, `Firefox`, `Chromium`, `VLC`, `MPV`, etc.).
   - Enrutamiento selectivo por aplicación: el usuario decide exactamente qué aplicaciones se ecualizan a través de la cadena DSP y cuáles pasan en *bypass* directo hacia el hardware.
2. **Rediseño Integral de la Interfaz Gráfica Iced:**
   - Erradicar la apariencia genérica y plana de sliders crudos.
   - Implementar la arquitectura visual **Obsidian Rack Acoustic** de 19 pulgadas descrita en `DESIGN.md`: bahías de faders maquinadas con marcas de escala en dB, perillas de hardware, medidores VU y panel de enrutamiento de reproductores.
3. **Desacoplamiento de Dependencias de Windows:**
   - Asegurar compilación limpia en Linux mediante aislamiento condicional (`#[cfg(target_os = "windows")]` vs `#[cfg(target_os = "linux")]`).

---

## 🔌 2. Arquitectura de Audio PipeWire

```
[ Aplicaciones del Sistema ]
       │
       ├── Spotify / Firefox (Seleccionados [x])
       │         │
       │         ▼
       │   [ Virtual Sink / Nodo Filtro PipeWire: "Aldra DSP Equalizer" ]
       │         │
       │         ▼
       │   [ Cadena Maestra DSP en Rust (Filtros Biquad + Tono + Compresor + Reverb) ]
       │         │
       │         ▼
       │   [ Hardware Output / Audífonos (JBL Tune / Altavoces) ]
       │
       └── Discord / Juegos / Notificaciones (En Bypass [ ])
                 │
                 └────────────── Directo ───────────────────────► [ Hardware Output ]
```

### Componentes de PipeWire:
- **`enrutador_pipewire.rs`:**
  - Consulta del grafo de nodos de PipeWire mediante `pw-dump Node` estructurado en JSON.
  - Extracción de metadatos: `props.application.name`, `props.media.name`, `node.id`, `node.name`.
  - Creación y enlace de nodos de filtro con `pw-link` o virtual loopback sink.
  - Detección de desconexión o apertura de nuevos reproductores sin reiniciar el motor.

---

## 🎨 3. Rediseño de la Interfaz Iced (Obsidian Rack Acoustic)

### Paleta de Materiales:
- **Chasis Base:** `#0d1117` (Obsidiana Mate de baja reflectancia).
- **Módulos / Bahías:** `#161b22` (Consola Slate de 19 pulgadas).
- **Faders / Contenedores:** `#21262d` con bordes `#30363d` y línea de cero central en `#8b949e`.
- **Acentos Activos:** `#f59e0b` (Ámbar Sunlight) para ecualización activa y `#38bdf8` (Cian de estudio) para curvas y análisis FFT.
- **Tipografía:** `JetBrains Mono` con alineación tabular para decibeles y hercios; `Roboto / Montserrat` para etiquetas.

### Nuevas Secciones de la UI:
1. **Bahía de Selección de Reproductores (PipeWire Stream Selector):**
   - Tarjeta Bento con interruptores individuales por aplicación.
   - Estado de reproducción en vivo.
2. **Bahía de Canal Paramétrica (Fader Bay):**
   - Lecturas exactas en dB (+12 a -24 dB con paso de 0.1 dB).
   - Interruptores de bypass por banda.
3. **Rack Dinámico (Compresor + Espacio + Reverb):**
   - Indicadores analógicos tipo LED ladder para reducción de ganancia (Gain Reduction).
