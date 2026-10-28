#pragma once

#include <vector>
#include <cmath>
#include <numbers>

enum class TipoFiltro {
    CAMPANA,        // Peaking EQ
    ESTANTERIA_BAJA, // Low Shelf
    ESTANTERIA_ALTA  // High Shelf
};

class FiltroBiquad {
private:
    double b0, b1, b2, a1, a2;
    double x1_izquierdo, x2_izquierdo, y1_izquierdo, y2_izquierdo;
    double x1_derecho, x2_derecho, y1_derecho, y2_derecho;
    
    double frecuencia_muestreo;
    double frecuencia_corte;
    double ganancia_decibelios;
    double factor_q;
    TipoFiltro tipo;

    void calcular_coeficientes();

public:
    FiltroBiquad();
    
    void configurar(TipoFiltro tipo_filtro, double frecuencia, double ganancia_db, double q, double fs);
    void actualizar_ganancia(double nueva_ganancia_db);
    void procesar_muestra_estereo(double& muestra_izquierda, double& muestra_derecha);
    void reiniciar_estado();
};

class EcualizadorDSP {
private:
    std::vector<FiltroBiquad> bandas;
    FiltroBiquad control_graves;
    FiltroBiquad control_agudos;
    
    double pre_amplificacion_lineal;
    double frecuencia_muestreo;

public:
    EcualizadorDSP();

    void inicializar(double fs);
    void establecer_pre_amplificacion_db(double ganancia_db);
    void establecer_ganancia_banda(int indice_banda, double ganancia_db);
    void establecer_tono(double ganancia_graves_db, double ganancia_agudos_db);
    
    // Procesa un buffer de audio estéreo intercalado (L, R, L, R...)
    void procesar_buffer(float* buffer, int cantidad_muestras);
};
