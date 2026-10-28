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
                text: "COMPRESOR (DINÁMICA)"
                color: "#ff2a5f"
                font.family: "Space Grotesk"
                font.pointSize: 18
                font.bold: true
                Layout.alignment: Qt.AlignHCenter
            }

            Text {
                text: "Requiere plugin ReaComp.dll en VSTPlugins"
                color: "#ffb347"
                font.family: "Inter"
                font.pointSize: 10
                font.italic: true
                Layout.alignment: Qt.AlignHCenter
            }

            RowLayout {
                spacing: 20
                Layout.alignment: Qt.AlignHCenter

                // Threshold
                ColumnLayout {
                    spacing: 10
                    Layout.alignment: Qt.AlignHCenter
                    Dial {
                        id: dialThresh
                        from: -60.0
                        to: 0.0
                        value: 0.0
                        Layout.preferredWidth: 80
                        Layout.preferredHeight: 80
                        onValueChanged: actualizarComp()
                    }
                    Text {
                        text: "Límite\n" + dialThresh.value.toFixed(1) + " dB"
                        color: "#ffffff"
                        font.family: "Inter"
                        font.pointSize: 9
                        horizontalAlignment: Text.AlignHCenter
                        Layout.alignment: Qt.AlignHCenter
                    }
                }

                // Ratio
                ColumnLayout {
                    spacing: 10
                    Layout.alignment: Qt.AlignHCenter
                    Dial {
                        id: dialRatio
                        from: 1.0
                        to: 20.0
                        value: 1.0
                        Layout.preferredWidth: 80
                        Layout.preferredHeight: 80
                        onValueChanged: actualizarComp()
                    }
                    Text {
                        text: "Proporción\n" + dialRatio.value.toFixed(1) + ":1"
                        color: "#ffffff"
                        font.family: "Inter"
                        font.pointSize: 9
                        horizontalAlignment: Text.AlignHCenter
                        Layout.alignment: Qt.AlignHCenter
                    }
                }

                // Attack
                ColumnLayout {
                    spacing: 10
                    Layout.alignment: Qt.AlignHCenter
                    Dial {
                        id: dialAttack
                        from: 0.1
                        to: 200.0
                        value: 10.0
                        Layout.preferredWidth: 80
                        Layout.preferredHeight: 80
                        onValueChanged: actualizarComp()
                    }
                    Text {
                        text: "Ataque\n" + dialAttack.value.toFixed(1) + " ms"
                        color: "#ffffff"
                        font.family: "Inter"
                        font.pointSize: 9
                        horizontalAlignment: Text.AlignHCenter
                        Layout.alignment: Qt.AlignHCenter
                    }
                }

                // Release
                ColumnLayout {
                    spacing: 10
                    Layout.alignment: Qt.AlignHCenter
                    Dial {
                        id: dialRelease
                        from: 10.0
                        to: 1000.0
                        value: 100.0
                        Layout.preferredWidth: 80
                        Layout.preferredHeight: 80
                        onValueChanged: actualizarComp()
                    }
                    Text {
                        text: "Lanzamiento\n" + dialRelease.value.toFixed(0) + " ms"
                        color: "#ffffff"
                        font.family: "Inter"
                        font.pointSize: 9
                        horizontalAlignment: Text.AlignHCenter
                        Layout.alignment: Qt.AlignHCenter
                    }
                }

                // Makeup
                ColumnLayout {
                    spacing: 10
                    Layout.alignment: Qt.AlignHCenter
                    Dial {
                        id: dialMakeup
                        from: -12.0
                        to: 12.0
                        value: 0.0
                        Layout.preferredWidth: 80
                        Layout.preferredHeight: 80
                        onValueChanged: actualizarComp()
                    }
                    Text {
                        text: "Ganancia\n" + dialMakeup.value.toFixed(1) + " dB"
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

    function actualizarComp() {
        if (controladorAudio) {
            controladorAudio.cambiar_compresor(dialThresh.value, dialRatio.value, dialAttack.value, dialRelease.value, dialMakeup.value)
        }
    }
}
