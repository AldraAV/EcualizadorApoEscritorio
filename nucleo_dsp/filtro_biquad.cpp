#include "filtro_biquad.hpp"
#include <numbers>
#include <algorithm>

namespace aldra::audio {

FiltroBiquad::FiltroBiquad()
    : coeficiente_b0(1.0), coeficiente_b1(0.0), coeficiente_b2(0.0),
      coeficiente_a1(0.0), coeficiente_a2(0.0),
      registro_estado_1(0.0), registro_estado_2(0.0),
      esta_activo(false) {
}

void FiltroBiquad::reiniciar_memoria() noexcept {
    registro_estado_1 = 0.0;
    registro_estado_2 = 0.0;
}

double FiltroBiquad::procesar_muestra(double muestra_entrada) noexcept {
    if (!esta_activo) {
        return muestra_entrada;
    }

    // Topología Direct Form II Transposed: mínima susceptibilidad a errores de redondeo en punto flotante
    const double muestra_salida = coeficiente_b0 * muestra_entrada + registro_estado_1;
    registro_estado_1 = coeficiente_b1 * muestra_entrada - coeficiente_a1 * muestra_salida + registro_estado_2;
    registro_estado_2 = coeficiente_b2 * muestra_entrada - coeficiente_a2 * muestra_salida;

    return muestra_salida;
}

void FiltroBiquad::configurar_banda(const DatosBanda& banda, double frecuencia_muestreo) {
    if (frecuencia_muestreo <= 0.0) {
        throw ParametroAudioInvalido("La frecuencia de muestreo del sistema debe ser mayor que cero.");
    }

    if (banda.frecuencia_hz >= frecuencia_muestreo * 0.5) {
        throw FrecuenciaFueraDeRango(banda.frecuencia_hz);
    }

    esta_activo = banda.activo;
    if (!esta_activo) {
        coeficiente_b0 = 1.0;
        coeficiente_b1 = 0.0;
        coeficiente_b2 = 0.0;
        coeficiente_a1 = 0.0;
        coeficiente_a2 = 0.0;
        return;
    }

    const double omega = 2.0 * std::numbers::pi_v<double> * banda.frecuencia_hz / frecuencia_muestreo;
    const double seno_omega = std::sin(omega);
    const double coseno_omega = std::cos(omega);
    const double alfa = seno_omega / (2.0 * banda.factor_q);
    const double factor_a = std::pow(10.0, banda.ganancia_db / 40.0); // A para estantes y pico

    double b0_crudo = 1.0, b1_crudo = 0.0, b2_crudo = 0.0;
    double a0_crudo = 1.0, a1_crudo = 0.0, a2_crudo = 0.0;

    switch (banda.tipo) {
        case TipoFiltro::PicoParametrico: {
            // Filtro Peaking EQ con ganancia modificable en decibelios
            b0_crudo = 1.0 + alfa * factor_a;
            b1_crudo = -2.0 * coseno_omega;
            b2_crudo = 1.0 - alfa * factor_a;
            a0_crudo = 1.0 + alfa / factor_a;
            a1_crudo = -2.0 * coseno_omega;
            a2_crudo = 1.0 - alfa / factor_a;
            break;
        }
        case TipoFiltro::EstanteGraves: {
            // Filtro Low Shelf (Estante de Graves)
            const double dos_raiz_a_alfa = 2.0 * std::sqrt(factor_a) * alfa;
            b0_crudo = factor_a * ((factor_a + 1.0) - (factor_a - 1.0) * coseno_omega + dos_raiz_a_alfa);
            b1_crudo = 2.0 * factor_a * ((factor_a - 1.0) - (factor_a + 1.0) * coseno_omega);
            b2_crudo = factor_a * ((factor_a + 1.0) - (factor_a - 1.0) * coseno_omega - dos_raiz_a_alfa);
            a0_crudo = (factor_a + 1.0) + (factor_a - 1.0) * coseno_omega + dos_raiz_a_alfa;
            a1_crudo = -2.0 * ((factor_a - 1.0) + (factor_a + 1.0) * coseno_omega);
            a2_crudo = (factor_a + 1.0) + (factor_a - 1.0) * coseno_omega - dos_raiz_a_alfa;
            break;
        }
        case TipoFiltro::EstanteAgudos: {
            // Filtro High Shelf (Estante de Agudos)
            const double dos_raiz_a_alfa = 2.0 * std::sqrt(factor_a) * alfa;
            b0_crudo = factor_a * ((factor_a + 1.0) + (factor_a - 1.0) * coseno_omega + dos_raiz_a_alfa);
            b1_crudo = -2.0 * factor_a * ((factor_a - 1.0) + (factor_a + 1.0) * coseno_omega);
            b2_crudo = factor_a * ((factor_a + 1.0) + (factor_a - 1.0) * coseno_omega - dos_raiz_a_alfa);
            a0_crudo = (factor_a + 1.0) - (factor_a - 1.0) * coseno_omega + dos_raiz_a_alfa;
            a1_crudo = 2.0 * ((factor_a - 1.0) - (factor_a + 1.0) * coseno_omega);
            a2_crudo = (factor_a + 1.0) - (factor_a - 1.0) * coseno_omega - dos_raiz_a_alfa;
            break;
        }
        case TipoFiltro::PasoAlto: {
            // Filtro Paso Alto (HPF) 12 dB/octava
            b0_crudo = (1.0 + coseno_omega) * 0.5;
            b1_crudo = -(1.0 + coseno_omega);
            b2_crudo = (1.0 + coseno_omega) * 0.5;
            a0_crudo = 1.0 + alfa;
            a1_crudo = -2.0 * coseno_omega;
            a2_crudo = 1.0 - alfa;
            break;
        }
        case TipoFiltro::PasoBajo: {
            // Filtro Paso Bajo (LPF) 12 dB/octava
            b0_crudo = (1.0 - coseno_omega) * 0.5;
            b1_crudo = 1.0 - coseno_omega;
            b2_crudo = (1.0 - coseno_omega) * 0.5;
            a0_crudo = 1.0 + alfa;
            a1_crudo = -2.0 * coseno_omega;
            a2_crudo = 1.0 - alfa;
            break;
        }
    }

    if (std::abs(a0_crudo) < 1e-15) {
        throw ErrorCalculoDsp("Inestabilidad numérica detectada en el denominador del filtro biquad (a0 cercano a cero).");
    }

    // Normalizar todos los coeficientes dividiéndolos por a0 para que el denominador empiece en 1.0
    const double inverso_a0 = 1.0 / a0_crudo;
    coeficiente_b0 = b0_crudo * inverso_a0;
    coeficiente_b1 = b1_crudo * inverso_a0;
    coeficiente_b2 = b2_crudo * inverso_a0;
    coeficiente_a1 = a1_crudo * inverso_a0;
    coeficiente_a2 = a2_crudo * inverso_a0;
}

double FiltroBiquad::evaluar_magnitud(double frecuencia_hz, double frecuencia_muestreo) const {
    if (!esta_activo || frecuencia_muestreo <= 0.0) {
        return 1.0;
    }

    // Fórmula exacta descubierta en com/maxmpz/audioplayer/equalizer/Coefs.smali -> método В()
    const double omega = 2.0 * std::numbers::pi_v<double> * frecuencia_hz / frecuencia_muestreo;
    const double seno_mitad_omega = std::sin(omega * 0.5);
    const double ese = seno_mitad_omega * seno_mitad_omega; // S en la fórmula smali

    const double suma_b = coeficiente_b0 + coeficiente_b1 + coeficiente_b2;
    const double suma_a = 1.0 + coeficiente_a1 + coeficiente_a2;

    const double num_base = suma_b * suma_b;
    const double prod_b = coeficiente_b0 * coeficiente_b1 + 4.0 * coeficiente_b0 * coeficiente_b2 + coeficiente_b1 * coeficiente_b2;
    const double num = num_base - 4.0 * prod_b * ese + 16.0 * coeficiente_b0 * coeficiente_b2 * ese * ese;

    const double den_base = suma_a * suma_a;
    const double prod_a = coeficiente_a1 + 4.0 * coeficiente_a2 + coeficiente_a1 * coeficiente_a2;
    const double den = den_base - 4.0 * prod_a * ese + 16.0 * coeficiente_a2 * ese * ese;

    if (std::abs(den) < 1e-15) {
        return 0.0;
    }

    const double cociente = num / den;
    return std::sqrt(std::max(0.0, cociente));
}

} // namespace aldra::audio
