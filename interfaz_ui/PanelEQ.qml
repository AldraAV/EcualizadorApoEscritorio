import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    // Panel Central: Perillas a la izquierda, Sliders a la derecha
    RowLayout {
        anchors.fill: parent
        spacing: 30

        // Panel de Perillas (Preamp, Bass, Treble)
        Rectangle {
            Layout.preferredWidth: 250
            Layout.fillHeight: true
            color: "#0Affffff"
            border.color: "#1Affffff"
            radius: 10
            
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 20
                spacing: 20

                Text {
                    text: "MASTERS"
                    color: "#ff2a5f"
                    font.family: "Space Grotesk"
                    font.pointSize: 14
                    font.bold: true
                    Layout.alignment: Qt.AlignHCenter
                }

                // Perilla Preamp
                ColumnLayout {
                    Layout.alignment: Qt.AlignHCenter
                    spacing: 5
                    Dial {
                        id: dialPreamp
                        from: -15.0
                        to: 15.0
                        value: 0.0
                        Layout.preferredWidth: 80
                        Layout.preferredHeight: 80
                        
                        onValueChanged: {
                            if (controladorAudio && !controladorAudio.actualizandoDesdeArchivo) {
                                controladorAudio.cambiar_ganancia_preamp(value)
                            }
                        }

                        Connections {
                            target: controladorAudio
                            function onPreampExternaCambiado() {
                                if (controladorAudio.actualizandoDesdeArchivo) {
                                    dialPreamp.value = controladorAudio.preampExterna
                                }
                            }
                        }
                    }
                    Text {
                        text: "Preamp\n" + dialPreamp.value.toFixed(1) + " dB"
                        color: "#ffffff"
                        font.family: "Inter"
                        font.pointSize: 9
                        horizontalAlignment: Text.AlignHCenter
                        Layout.alignment: Qt.AlignHCenter
                    }
                }

                // Fila para Bass / Treble
                RowLayout {
                    Layout.alignment: Qt.AlignHCenter
                    spacing: 20
                    
                    ColumnLayout {
                        spacing: 5
                        Dial {
                            id: dialBass
                            from: -12.0
                            to: 12.0
                            value: 0.0
                            Layout.preferredWidth: 60
                            Layout.preferredHeight: 60
                            onValueChanged: {
                                if (controladorAudio && !controladorAudio.actualizandoDesdeArchivo) {
                                    controladorAudio.cambiar_tono(dialBass.value, dialTreble.value)
                                }
                            }

                            Connections {
                                target: controladorAudio
                                function onTonoExternoCambiado() {
                                    if (controladorAudio.actualizandoDesdeArchivo) {
                                        dialBass.value = controladorAudio.gravesExterno
                                    }
                                }
                            }
                        }
                        Text {
                            text: "Graves\n" + dialBass.value.toFixed(1) + " dB"
                            color: "#ffffff"
                            font.family: "Inter"
                            font.pointSize: 8
                            horizontalAlignment: Text.AlignHCenter
                            Layout.alignment: Qt.AlignHCenter
                        }
                    }

                    ColumnLayout {
                        spacing: 5
                        Dial {
                            id: dialTreble
                            from: -12.0
                            to: 12.0
                            value: 0.0
                            Layout.preferredWidth: 60
                            Layout.preferredHeight: 60
                            onValueChanged: {
                                if (controladorAudio && !controladorAudio.actualizandoDesdeArchivo) {
                                    controladorAudio.cambiar_tono(dialBass.value, dialTreble.value)
                                }
                            }

                            Connections {
                                target: controladorAudio
                                function onTonoExternoCambiado() {
                                    if (controladorAudio.actualizandoDesdeArchivo) {
                                        dialTreble.value = controladorAudio.agudosExterno
                                    }
                                }
                            }
                        }
                        Text {
                            text: "Agudos\n" + dialTreble.value.toFixed(1) + " dB"
                            color: "#ffffff"
                            font.family: "Inter"
                            font.pointSize: 8
                            horizontalAlignment: Text.AlignHCenter
                            Layout.alignment: Qt.AlignHCenter
                        }
                    }
                }
            }
        }

        // Sliders del Ecualizador Paramétrico (10 bandas)
        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            color: "#0Affffff"
            border.color: "#1Affffff"
            radius: 10

            RowLayout {
                anchors.fill: parent
                anchors.margins: 10
                spacing: 8

                property var fcs: [31.5, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000]
                property var etiquetas: ["31", "63", "125", "250", "500", "1k", "2k", "4k", "8k", "16k"]

                Repeater {
                    model: 10
                    ColumnLayout {
                        Layout.alignment: Qt.AlignHCenter | Qt.AlignBottom
                        spacing: 5

                        // Control Q (Ancho de banda)
                        Dial {
                            id: dialQ
                            from: 0.1
                            to: 10.0
                            value: 1.41
                            Layout.preferredWidth: 40
                            Layout.preferredHeight: 40
                            Layout.alignment: Qt.AlignHCenter
                            onValueChanged: {
                                if (controladorAudio && !controladorAudio.actualizandoDesdeArchivo) {
                                    actualizarFiltro()
                                }
                            }
                        }
                        Text {
                            text: "Q:" + dialQ.value.toFixed(1)
                            color: "#aaaaaa"
                            font.family: "JetBrains Mono"
                            font.pointSize: 7
                            Layout.alignment: Qt.AlignHCenter
                        }

                        // Ganancia
                        Text {
                            text: sliderGanancia.value.toFixed(1) + " dB"
                            color: "#ffffff"
                            font.family: "Inter"
                            font.pointSize: 9
                            Layout.alignment: Qt.AlignHCenter
                        }

                        Slider {
                            id: sliderGanancia
                            Layout.fillHeight: true
                            orientation: Qt.Vertical
                            from: -12.0
                            to: 12.0
                            value: 0.0
                            onValueChanged: {
                                if (controladorAudio && !controladorAudio.actualizandoDesdeArchivo) {
                                    actualizarFiltro()
                                }
                            }
                        }

                        // Frecuencia
                        Text {
                            text: parent.parent.etiquetas[index]
                            color: "#ff2a5f"
                            font.family: "JetBrains Mono"
                            font.pointSize: 10
                            font.bold: true
                            Layout.alignment: Qt.AlignHCenter
                        }

                        function actualizarFiltro() {
                            if (controladorAudio) {
                                var fc = parent.parent.fcs[index];
                                controladorAudio.cambiar_param_eq(index, fc, sliderGanancia.value, dialQ.value)
                            }
                        }

                        Connections {
                            target: controladorAudio
                            function onGananciasExternaCambiadas() {
                                if (controladorAudio.actualizandoDesdeArchivo) {
                                    var gains = controladorAudio.gananciasExterna
                                    if (gains && index < gains.length) {
                                        sliderGanancia.value = gains[index]
                                    }
                                    var qs = controladorAudio.qsExterna
                                    if (qs && index < qs.length) {
                                        dialQ.value = qs[index]
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
