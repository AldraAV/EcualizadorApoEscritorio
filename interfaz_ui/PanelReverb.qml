import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    Rectangle {
        anchors.fill: parent
        color: "#0Affffff"
        border.color: "#1Affffff"
        radius: 10

        ColumnLayout {
            anchors.centerIn: parent
            spacing: 30

            Text {
                text: "PROCESADOR DE REVERBERACIÓN"
                color: "#ff2a5f"
                font.family: "Space Grotesk"
                font.pointSize: 18
                font.bold: true
                Layout.alignment: Qt.AlignHCenter
            }

            Text {
                text: "Requiere plugin ReaVerbate.dll en VSTPlugins"
                color: "#ffb347"
                font.family: "Inter"
                font.pointSize: 10
                font.italic: true
                Layout.alignment: Qt.AlignHCenter
            }

            RowLayout {
                spacing: 30
                Layout.alignment: Qt.AlignHCenter

                // Size
                ColumnLayout {
                    spacing: 10
                    Layout.alignment: Qt.AlignHCenter
                    Dial {
                        id: dialSize
                        from: 0.0
                        to: 1.0
                        value: 0.5
                        Layout.preferredWidth: 90
                        Layout.preferredHeight: 90
                        onValueChanged: actualizarReverb()
                    }
                    Text {
                        text: "Tamaño\n" + (dialSize.value * 100).toFixed(0) + "%"
                        color: "#ffffff"
                        font.family: "Inter"
                        font.pointSize: 9
                        horizontalAlignment: Text.AlignHCenter
                        Layout.alignment: Qt.AlignHCenter
                    }
                }

                // Pre-delay
                ColumnLayout {
                    spacing: 10
                    Layout.alignment: Qt.AlignHCenter
                    Dial {
                        id: dialPredelay
                        from: 0.0
                        to: 100.0
                        value: 0.0
                        Layout.preferredWidth: 90
                        Layout.preferredHeight: 90
                        onValueChanged: actualizarReverb()
                    }
                    Text {
                        text: "Preretardo\n" + dialPredelay.value.toFixed(0) + " ms"
                        color: "#ffffff"
                        font.family: "Inter"
                        font.pointSize: 9
                        horizontalAlignment: Text.AlignHCenter
                        Layout.alignment: Qt.AlignHCenter
                    }
                }

                // Damping
                ColumnLayout {
                    spacing: 10
                    Layout.alignment: Qt.AlignHCenter
                    Dial {
                        id: dialDamping
                        from: 0.0
                        to: 1.0
                        value: 0.5
                        Layout.preferredWidth: 90
                        Layout.preferredHeight: 90
                        onValueChanged: actualizarReverb()
                    }
                    Text {
                        text: "Amortiguación\n" + (dialDamping.value * 100).toFixed(0) + "%"
                        color: "#ffffff"
                        font.family: "Inter"
                        font.pointSize: 9
                        horizontalAlignment: Text.AlignHCenter
                        Layout.alignment: Qt.AlignHCenter
                    }
                }

                // Mix (Dry/Wet)
                ColumnLayout {
                    spacing: 10
                    Layout.alignment: Qt.AlignHCenter
                    Dial {
                        id: dialMix
                        from: 0.0
                        to: 1.0
                        value: 0.0
                        Layout.preferredWidth: 90
                        Layout.preferredHeight: 90
                        onValueChanged: actualizarReverb()
                    }
                    Text {
                        text: "Mezclado\n" + (dialMix.value * 100).toFixed(0) + "%"
                        color: "#ffffff"
                        font.family: "Inter"
                        font.pointSize: 9
                        horizontalAlignment: Text.AlignHCenter
                        Layout.alignment: Qt.AlignHCenter
                    }
                }
            }
        }
    }

    function actualizarReverb() {
        if (controladorAudio) {
            controladorAudio.cambiar_reverb(dialSize.value, dialPredelay.value, dialDamping.value, dialMix.value)
        }
    }
}
