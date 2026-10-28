#include "ecualizador_parametrico.hpp"
#include <algorithm>
#include <cmath>

namespace aldra::audio {

EcualizadorParametrico::EcualizadorParametrico(double frecuencia_muestreo)
    : frecuencia_muestreo_actual(frecuencia_muestreo) {
    if (frecuencia_muestreo_actual <= 0.0) {
        throw ParametroAudioInvalido("Frecuencia de muestreo debe ser positiva al inicializar el Ecualizador.");
    }
}

void EcualizadorParametrico::establecer_frecuencia_muestreo(double nueva_frecuencia_muestreo) {
    std::lock_guard<std::mutex> candado(mutex_parametros);
    if (nueva_frecuencia_muestreo <= 0.0) {
        throw ParametroAudioInvalido("Nueva frecuencia de muestreo inválida.");
    }
    frecuencia_muestreo_actual = nueva_frecuencia_muestreo;

    for (size_t indice = 0; indice < lista_bandas.size(); ++indice) {
        filtros_canal_izquierdo[indice].configurar_banda(lista_bandas[indice], frecuencia_muestreo_actual);
        filtros_canal_derecho[indice].configurar_banda(lista_bandas[indice], frecuencia_muestreo_actual);
    }
}

void EcualizadorParametrico::configurar_banda(const DatosBanda& banda) {
    std::lock_guard<std::mutex> candado(mutex_parametros);
    
    // Buscar si la banda ya existe
    auto iterador = std::find_if(lista_bandas.begin(), lista_bandas.end(),
        [&banda](const DatosBanda& b) { return b.identificador == banda.identificador; });

    if (iterador != lista_bandas.end()) {
        const size_t indice = std::distance(lista_bandas.begin(), iterador);
        lista_bandas[indice] = banda;
        filtros_canal_izquierdo[indice].configurar_banda(banda, frecuencia_muestreo_actual);
        filtros_canal_derecho[indice].configurar_banda(banda, frecuencia_muestreo_actual);
    } else {
        lista_bandas.push_back(banda);
        FiltroBiquad filtro_izq, filtro_der;
        filtro_izq.configurar_banda(banda, frecuencia_muestreo_actual);
        filtro_der.configurar_banda(banda, frecuencia_muestreo_actual);
        filtros_canal_izquierdo.push_back(filtro_izq);
        filtros_canal_derecho.push_back(filtro_der);
    }
}

void EcualizadorParametrico::eliminar_banda(int identificador) {
    std::lock_guard<std::mutex> candado(mutex_parametros);
    for (size_t indice = 0; indice < lista_bandas.size(); ++indice) {
        if (lista_bandas[indice].identificador == identificador) {
            lista_bandas.erase(lista_bandas.begin() + indice);
            filtros_canal_izquierdo.erase(filtros_canal_izquierdo.begin() + indice);
            filtros_canal_derecho.erase(filtros_canal_derecho.begin() + indice);
            break;
        }
    }
}

void EcualizadorParametrico::reiniciar_historial_filtros() noexcept {
    std::lock_guard<std::mutex> candado(mutex_parametros);
    for (auto& filtro : filtros_canal_izquierdo) {
        filtro.reiniciar_memoria();
    }
    for (auto& filtro : filtros_canal_derecho) {
        filtro.reiniciar_memoria();
    }
}

void EcualizadorParametrico::procesar_buffer_estereo(float* buffer, size_t numero_muestras) noexcept {
    if (buffer == nullptr || numero_muestras == 0) {
        return;
    }

    // Intentamos adquirir el candado sin bloquear de forma indefinida en el hilo de audio en tiempo real
    if (!mutex_parametros.try_lock()) {
        // Si hay una reconfiguración rápida en proceso, devolvemos el audio sin alterar para no generar latencia ni clics
        return;
    }

    const size_t cantidad_bandas = lista_bandas.size();
    for (size_t indice_muestra = 0; indice_muestra < numero_muestras; indice_muestra += 2) {
        // Promoción a double (64 bits) para procesamiento numérico de alta estabilidad
        double muestra_izq = static_cast<double>(buffer[indice_muestra]);
        double muestra_der = static_cast<double>(buffer[indice_muestra + 1]);

        for (size_t indice_banda = 0; indice_banda < cantidad_bandas; ++indice_banda) {
            if (!lista_bandas[indice_banda].activo) {
                continue;
            }

            const int canales = lista_bandas[indice_banda].canales_afectados;
            if (canales == 0 || canales == 1) {
                muestra_izq = filtros_canal_izquierdo[indice_banda].procesar_muestra(muestra_izq);
            }
            if (canales == 0 || canales == 2) {
                muestra_der = filtros_canal_derecho[indice_banda].procesar_muestra(muestra_der);
            }
        }

        // Limitación suave/saturación segura de punto flotante para no causar distorsión digital severa
        buffer[indice_muestra] = static_cast<float>(std::clamp(muestra_izq, -1.0, 1.0));
        buffer[indice_muestra + 1] = static_cast<float>(std::clamp(muestra_der, -1.0, 1.0));
    }

    mutex_parametros.unlock();
}

double EcualizadorParametrico::evaluar_respuesta_db(double frecuencia_hz) const {
    std::lock_guard<std::mutex> candado(mutex_parametros);
    double magnitud_acumulada = 1.0;

    for (size_t indice = 0; indice < lista_bandas.size(); ++indice) {
        if (lista_bandas[indice].activo) {
            magnitud_acumulada *= filtros_canal_izquierdo[indice].evaluar_magnitud(frecuencia_hz, frecuencia_muestreo_actual);
        }
    }

    if (magnitud_acumulada <= 0.0) {
        return -120.0; // Silencio práctico
    }

    return 20.0 * std::log10(magnitud_acumulada);
}

} // namespace aldra::audio
