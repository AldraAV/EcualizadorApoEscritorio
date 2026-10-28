#include <pybind11/pybind11.h>
#include <pybind11/stl.h>
#include "../nucleo_dsp/motor_audio.h"

namespace py = pybind11;

PYBIND11_MODULE(motor_audio_nativo, m) {
    m.doc() = "Modulo nativo del Motor de Audio DSP";

    py::class_<DatosEspectro>(m, "DatosEspectro")
        .def_readonly("magnitudes", &DatosEspectro::magnitudes)
        .def_readonly("actualizados", &DatosEspectro::actualizados);

    py::class_<MotorAudio>(m, "MotorAudio")
        .def(py::init<>())
        .def("inicializar_dispositivo", &MotorAudio::inicializar_dispositivo)
        .def("iniciar", &MotorAudio::iniciar)
        .def("detener", &MotorAudio::detener)
        .def("obtener_datos_espectro", &MotorAudio::obtener_datos_espectro);
}
