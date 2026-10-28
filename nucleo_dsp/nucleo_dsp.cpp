#include "nucleo_dsp.h"

FiltroBiquad::FiltroBiquad() {
    b0 = b1 = b2 = 0.0;
    a1 = a2 = 0.0;
    reiniciar_estado();
}

void FiltroBiquad::reiniciar_estado() {
    x1_izquierdo = x2_izquierdo = y1_izquierdo = y2_izquierdo = 0.0;
    x1_derecho = x2_derecho = y1_derecho = y2_derecho = 0.0;
}

void FiltroBiquad::configurar(TipoFiltro tipo_filtro, double frecuencia, double ganancia_db, double q, double fs) {
    tipo = tipo_filtro;
    frecuencia_corte = frecuencia;
    ganancia_decibelios = ganancia_db;
    factor_q = q;
    frecuencia_muestreo = fs;
    calcular_coeficientes();
}

void FiltroBiquad::actualizar_ganancia(double nueva_ganancia_db) {
    ganancia_decibelios = nueva_ganancia_db;
    calcular_coeficientes();
}

void FiltroBiquad::calcular_coeficientes() {
    double a0 = 1.0;
    double A = std::pow(10.0, ganancia_decibelios / 40.0);
    double w0 = 2.0 * std::numbers::pi * frecuencia_corte / frecuencia_muestreo;
    double alfa = std::sin(w0) / (2.0 * factor_q);
    double cos_w0 = std::cos(w0);

    if (tipo == TipoFiltro::CAMPANA) {
        b0 = 1.0 + alfa * A;
        b1 = -2.0 * cos_w0;
        b2 = 1.0 - alfa * A;
        a0 = 1.0 + alfa / A;
        a1 = -2.0 * cos_w0;
        a2 = 1.0 - alfa / A;
    } else if (tipo == TipoFiltro::ESTANTERIA_BAJA) {
        double raiz_A = std::sqrt(A);
        b0 = A * ((A + 1.0) - (A - 1.0) * cos_w0 + 2.0 * raiz_A * alfa);
        b1 = 2.0 * A * ((A - 1.0) - (A + 1.0) * cos_w0);
        b2 = A * ((A + 1.0) - (A - 1.0) * cos_w0 - 2.0 * raiz_A * alfa);
        a0 = (A + 1.0) + (A - 1.0) * cos_w0 + 2.0 * raiz_A * alfa;
        a1 = -2.0 * ((A - 1.0) + (A + 1.0) * cos_w0);
        a2 = (A + 1.0) + (A - 1.0) * cos_w0 - 2.0 * raiz_A * alfa;
    } else if (tipo == TipoFiltro::ESTANTERIA_ALTA) {
        double raiz_A = std::sqrt(A);
        b0 = A * ((A + 1.0) + (A - 1.0) * cos_w0 + 2.0 * raiz_A * alfa);
        b1 = -2.0 * A * ((A - 1.0) + (A + 1.0) * cos_w0);
        b2 = A * ((A + 1.0) + (A - 1.0) * cos_w0 - 2.0 * raiz_A * alfa);
        a0 = (A + 1.0) - (A - 1.0) * cos_w0 + 2.0 * raiz_A * alfa;
        a1 = 2.0 * ((A - 1.0) - (A + 1.0) * cos_w0);
        a2 = (A + 1.0) - (A - 1.0) * cos_w0 - 2.0 * raiz_A * alfa;
    }

    // Normalizar coeficientes dividiéndolos por a0
    b0 /= a0;
    b1 /= a0;
    b2 /= a0;
    a1 /= a0;
    a2 /= a0;
}

void FiltroBiquad::procesar_muestra_estereo(double& muestra_izquierda, double& muestra_derecha) {
    // Canal Izquierdo
    double salida_izq = b0 * muestra_izquierda + b1 * x1_izquierdo + b2 * x2_izquierdo - a1 * y1_izquierdo - a2 * y2_izquierdo;
    x2_izquierdo = x1_izquierdo;
    x1_izquierdo = muestra_izquierda;
    y2_izquierdo = y1_izquierdo;
    y1_izquierdo = salida_izq;
    muestra_izquierda = salida_izq;

    // Canal Derecho
    double salida_der = b0 * muestra_derecha + b1 * x1_derecho + b2 * x2_derecho - a1 * y1_derecho - a2 * y2_derecho;
    x2_derecho = x1_derecho;
    x1_derecho = muestra_derecha;
    y2_derecho = y1_derecho;
    y1_derecho = salida_der;
    muestra_derecha = salida_der;
}

// ==========================================
// Implementación del Ecualizador Completo
// ==========================================

EcualizadorDSP::EcualizadorDSP() {
    pre_amplificacion_lineal = 1.0;
    frecuencia_muestreo = 44100.0;
}

void EcualizadorDSP::inicializar(double fs) {
    frecuencia_muestreo = fs;
    bandas.resize(10);
    
    // Frecuencias estándar ISO para 10 bandas (octava completa)
    double frecuencias_bandas[10] = {31.25, 62.5, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0};
    
    for (int i = 0; i < 10; ++i) {
        bandas[i].configurar(TipoFiltro::CAMPANA, frecuencias_bandas[i], 0.0, 1.414, fs);
    }

    // Controles de tono (Graves alrededor de 100 Hz, Agudos alrededor de 10000 Hz)
    control_graves.configurar(TipoFiltro::ESTANTERIA_BAJA, 100.0, 0.0, 0.707, fs);
    control_agudos.configurar(TipoFiltro::ESTANTERIA_ALTA, 10000.0, 0.0, 0.707, fs);
}

void EcualizadorDSP::establecer_pre_amplificacion_db(double ganancia_db) {
    pre_amplificacion_lineal = std::pow(10.0, ganancia_db / 20.0);
}

void EcualizadorDSP::establecer_ganancia_banda(int indice_banda, double ganancia_db) {
    if (indice_banda >= 0 && indice_banda < 10) {
        bandas[indice_banda].actualizar_ganancia(ganancia_db);
    }
}

void EcualizadorDSP::establecer_tono(double ganancia_graves_db, double ganancia_agudos_db) {
    control_graves.actualizar_ganancia(ganancia_graves_db);
    control_agudos.actualizar_ganancia(ganancia_agudos_db);
}

void EcualizadorDSP::procesar_buffer(float* buffer, int cantidad_muestras) {
    // Procesamos en pares estéreo (Izquierdo, Derecho)
    for (int i = 0; i < cantidad_muestras; i += 2) {
        // 1. Aplicar preamplificación (conversión flotante a doble precisión para cálculos DSP)
        double muestra_izq = buffer[i] * pre_amplificacion_lineal;
        double muestra_der = buffer[i+1] * pre_amplificacion_lineal;
        
        // 2. Procesar las 10 bandas de ecualización en cascada
        for (auto& banda : bandas) {
            banda.procesar_muestra_estereo(muestra_izq, muestra_der);
        }
        
        // 3. Procesar controles de tono (Graves y Agudos)
        control_graves.procesar_muestra_estereo(muestra_izq, muestra_der);
        control_agudos.procesar_muestra_estereo(muestra_izq, muestra_der);
        
        // 4. Asignar de vuelta al buffer (limitación sencilla tipo clipping)
        if (muestra_izq > 1.0) muestra_izq = 1.0;
        else if (muestra_izq < -1.0) muestra_izq = -1.0;
        
        if (muestra_der > 1.0) muestra_der = 1.0;
        else if (muestra_der < -1.0) muestra_der = -1.0;
        
        buffer[i] = static_cast<float>(muestra_izq);
        buffer[i+1] = static_cast<float>(muestra_der);
    }
}
