# Especificación y Arquitectura Visual: Interfaz Aurora Ámbar Sunlight

## 1. Filosofía de Diseño: Esqueumorfismo Analógico + Claymorphism en Bento Grid
Siguiendo las directivas estrictas del Aldraverso y el **Purple Ban** (prohibición total de violetas y púrpuras), así como la eliminación del Glassmorphism (sin efecto vidrio flotante), la interfaz de escritorio del *Ecualizador Poweramp para Windows* se estructura bajo una **Spatial UI en Bento Grid** donde los controles físicos adquieren un volumen táctil inflado (Claymorphism) y un biselado de consola analógica real (Esqueumorfismo).

---

## 2. Paleta de Color Soberana: Aurora Ámbar Sunlight
* **Fondo Espacial Profundo:** `#0c0b09` a `#161410`. Un negro carbón cálido que elimina la fatiga ocular visual.
* **Luz Solar Cenital (Ámbar Sunlight):** `#f59e0b` y `#fbbf24`. Utilizado en la iluminación de los faders activos, visualizadores de espectro en vivo y medidores de ganancia en decibelios.
* **Lava Neón (Saturación/Clipping):** `#ef4444`. Se enciende únicamente cuando el nivel de audio en el búfer de salida supera los `0.0 dBFS`.

---

## 3. Santísima Trinidad Tipográfica (AldraAV)
1. **Space Grotesk (`--fuente-titulos`):** Títulos de los módulos en la Spatial UI (`ECUALIZADOR PARAMÉTRICO 10-BANDAS`, `COMPRESOR DINÁMICO`, `PERFILES DE AUDIO`).
2. **Inter (`--fuente-ui`):** Etiquetas de interruptores y menús desplegables.
3. **JetBrains Mono (`--fuente-datos`):** Pantallas digitales incrustadas y lecturas numéricas de alta precisión (`31.5 Hz`, `+4.5 dB`, `Q: 1.41`).
4. **Poppins (`--fuente-botones`):** Botones de acción principal cálidos y redondeados.

---

## 4. Componentes Clave (React + Framer Motion)
* **`TarjetasBentoClay`:** Módulos que flotan en un espacio 3D (`perspective: 1200px`) con sombras dobles que imitan un chasis acústico moldeado en arcilla y polímeros de alta densidad.
* **`FaderAnalogoEsqueumorfico`:** Deslizadores verticales con canal biselado profundo (`box-shadow: var(--bisel-fader-canal)`) y capuchón metálico/mate con línea indicadora ámbar radiante.
* **`PerillaRotativa3D`:** Potenciómetros con textura estriada y sombreado direccional que envían cambios continuos de parámetro al hilo de audio C++ sin truncamiento.

---

## 5. Enlace con el Núcleo C++ (`ecuacore` / APO)
La interfaz visual reacciona a **60 FPS** sincronizando su estado con el motor nativo de C++ (`nucleo_dsp`) que se ejecuta en el backend:
1. **Al arrastrar un capuchón Esqueumórfico en React:** `framer-motion` calcula la nueva ganancia en decibelios y la emite vía IPC/Tubería al módulo `EcualizadorParametrico::configurar_banda()`.
2. **Al recibir búferes del APO en tiempo real:** El núcleo envía la transformada rápida de Fourier (FFT) calculada sobre las muestras flotantes de 64 bits a las barras animadas color Ámbar del visualizador Bento Grid.
