import sys
import os
import json
import subprocess
import threading
import asyncio
from PySide6.QtGui import QGuiApplication
from PySide6.QtQml import QQmlApplicationEngine
import re
from PySide6.QtCore import QObject, Slot, Signal, Property, QTimer, QFileSystemWatcher

class ControladorAudio(QObject):
    magnitudesCambiadas = Signal()
    pistaCambiada = Signal()
    preampExternaCambiado = Signal()
    gananciasExternaCambiadas = Signal()
    tonoExternoCambiado = Signal()
    actualizandoDesdeArchivoCambiado = Signal()

    def __init__(self):
        super().__init__()
        self._reproduciendo = False
        self.lista_magnitudes = [0.0] * 64
        self.proceso_backend = None
        self.hilo_lectura = None
        self.ejecutando = True

        self._titulo_pista = ""
        self._artista_pista = ""
        self._app_pista = ""

        # Timer para actualizar la UI desde la variable interna sin bloquear
        self.timer = QTimer(self)
        self.timer.timeout.connect(self.actualizar_ui)

        self._fcs_bandas = [31.5, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000]
        self._preamp_externa = 0.0
        self._ganancias_bandas = [0.0] * 10
        self._qs_bandas = [1.41] * 10
        self._graves_externo = 0.0
        self._agudos_externo = 0.0
        self._actualizando_desde_archivo = False

        self.watcher = QFileSystemWatcher(self)
        self.archivo_apo = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "apo_config.txt"))
        if os.path.exists(self.archivo_apo):
            self.watcher.addPath(self.archivo_apo)
        self.watcher.fileChanged.connect(self.manejar_cambio_archivo_apo)
        
        self.manejar_cambio_archivo_apo(self.archivo_apo)

        self.hilo_media = threading.Thread(target=self.bucle_monitor_media, daemon=True)
        self.hilo_media.start()

    def bucle_monitor_media(self):
        import asyncio
        asyncio.run(self.monitor_media_async())

    async def monitor_media_async(self):
        try:
            from winsdk.windows.media.control import GlobalSystemMediaTransportControlsSessionManager
        except ImportError:
            return
            
        manager = await GlobalSystemMediaTransportControlsSessionManager.request_async()
        while self.ejecutando:
            try:
                session = manager.get_current_session()
                if session:
                    info = await session.try_get_media_properties_async()
                    titulo = info.title
                    artista = info.artist
                    app = session.source_app_user_model_id
                    
                    cambiado = False
                    if titulo != self._titulo_pista:
                        self._titulo_pista = titulo
                        cambiado = True
                    if artista != self._artista_pista:
                        self._artista_pista = artista
                        cambiado = True
                    if app != self._app_pista:
                        self._app_pista = app
                        cambiado = True
                        
                    if cambiado:
                        self.pistaCambiada.emit()
                else:
                    if self._titulo_pista != "":
                        self._titulo_pista = ""
                        self._artista_pista = ""
                        self._app_pista = ""
                        self.pistaCambiada.emit()
            except Exception:
                pass
            await asyncio.sleep(1)

    def iniciar_backend(self):
        backend_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "build", "motor_audio_backend.exe"))
        if not os.path.exists(backend_path):
            print(f"Error: No se encontro el ejecutable backend en {backend_path}")
            return
            
        self.proceso_backend = subprocess.Popen(
            [backend_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            bufsize=1
        )
        
        self.hilo_lectura = threading.Thread(target=self.leer_stdout, daemon=True)
        self.hilo_lectura.start()
        self._reproduciendo = True
        self.reproduciendoCambiado.emit()
        self.timer.start(16)

    def detener_backend(self):
        if self.proceso_backend:
            self.proceso_backend.terminate()
            self.proceso_backend.wait()
            self.proceso_backend = None
        self._reproduciendo = False
        self.reproduciendoCambiado.emit()
        self.timer.stop()
        self.lista_magnitudes = [0.0] * 64
        self.magnitudesCambiadas.emit()

    def leer_stdout(self):
        while self.ejecutando and self.proceso_backend:
            linea = self.proceso_backend.stdout.readline()
            if not linea:
                break
            try:
                if linea.startswith("{"):
                    datos = json.loads(linea)
                    if "magnitudes" in datos:
                        mags = datos["magnitudes"]
                        paso = max(1, len(mags) // 64)
                        nueva_lista = []
                        for i in range(64):
                            suma = 0.0
                            elementos = 0
                            for j in range(paso):
                                idx = i * paso + j
                                if idx < len(mags):
                                    suma += mags[idx]
                                    elementos += 1
                            promedio = suma / elementos if elementos > 0 else 0.0
                            nueva_lista.append(promedio)
                        
                        self.lista_magnitudes = nueva_lista
            except Exception as e:
                pass

    @Property(str, notify=pistaCambiada)
    def tituloPista(self):
        return self._titulo_pista

    @Property(str, notify=pistaCambiada)
    def artistaPista(self):
        return self._artista_pista

    @Property(str, notify=pistaCambiada)
    def appPista(self):
        return self._app_pista

    reproduciendoCambiado = Signal()

    @Slot()
    def alternar_reproduccion(self):
        if self._reproduciendo:
            self.detener_backend()
        else:
            self.iniciar_backend()

    def esta_reproduciendo(self):
        return self._reproduciendo

    reproduciendo = Property(bool, esta_reproduciendo, notify=reproduciendoCambiado)

    def leer_magnitudes(self):
        return self.lista_magnitudes

    magnitudes = Property(list, leer_magnitudes, notify=magnitudesCambiadas)

    @Slot(int, float)
    def cambiar_ganancia_banda(self, indice: int, ganancia_db: float):
        if self.proceso_backend and self.proceso_backend.stdin:
            try:
                self.proceso_backend.stdin.write(f"BANDA {indice} {ganancia_db}\n")
                self.proceso_backend.stdin.flush()
            except Exception as e:
                pass

    @Slot(float)
    def cambiar_ganancia_preamp(self, ganancia_db: float):
        if self.proceso_backend and self.proceso_backend.stdin:
            try:
                self.proceso_backend.stdin.write(f"PREAMP {ganancia_db}\n")
                self.proceso_backend.stdin.flush()
            except Exception as e:
                pass

    @Slot(float, float)
    def cambiar_tono(self, graves_db: float, agudos_db: float):
        if self.proceso_backend and self.proceso_backend.stdin:
            try:
                self.proceso_backend.stdin.write(f"TONO {graves_db} {agudos_db}\n")
                self.proceso_backend.stdin.flush()
            except Exception as e:
                pass

    @Slot(int, float, float, float)
    def cambiar_param_eq(self, indice: int, fc: float, ganancia_db: float, q: float):
        if self.proceso_backend and self.proceso_backend.stdin:
            try:
                self.proceso_backend.stdin.write(f"PARAM_BANDA {indice} {fc} {ganancia_db} {q}\n")
                self.proceso_backend.stdin.flush()
            except Exception as e:
                pass

    @Slot(float, float, int)
    def cambiar_espacio(self, balance: float, expand: float, mono: int):
        if self.proceso_backend and self.proceso_backend.stdin:
            try:
                self.proceso_backend.stdin.write(f"ESPACIO {balance} {expand} {mono}\n")
                self.proceso_backend.stdin.flush()
            except Exception as e:
                pass

    @Slot(float, float, float, float, float)
    def cambiar_compresor(self, thresh: float, ratio: float, attack: float, release: float, makeup: float):
        if self.proceso_backend and self.proceso_backend.stdin:
            try:
                self.proceso_backend.stdin.write(f"COMPRESOR {thresh} {ratio} {attack} {release} {makeup}\n")
                self.proceso_backend.stdin.flush()
            except Exception as e:
                pass

    @Slot(float, float, float, float)
    def cambiar_reverb(self, size: float, predelay: float, damping: float, mix: float):
        if self.proceso_backend and self.proceso_backend.stdin:
            try:
                self.proceso_backend.stdin.write(f"REVERB {size} {predelay} {damping} {mix}\n")
                self.proceso_backend.stdin.flush()
            except Exception as e:
                pass

    @Slot()
    def actualizar_ui(self):
        self.magnitudesCambiadas.emit()

    @Property(bool, notify=actualizandoDesdeArchivoCambiado)
    def actualizandoDesdeArchivo(self):
        return self._actualizando_desde_archivo
        
    @Property(float, notify=preampExternaCambiado)
    def preampExterna(self):
        return self._preamp_externa
        
    @Property(list, notify=gananciasExternaCambiadas)
    def gananciasExterna(self):
        return self._ganancias_bandas

    @Property(list, notify=gananciasExternaCambiadas)
    def qsExterna(self):
        return self._qs_bandas

    @Property(float, notify=tonoExternoCambiado)
    def gravesExterno(self):
        return self._graves_externo

    @Property(float, notify=tonoExternoCambiado)
    def agudosExterno(self):
        return self._agudos_externo

    @Slot(str)
    def manejar_cambio_archivo_apo(self, ruta: str):
        try:
            with open(ruta, "r", encoding="utf-8") as f:
                texto = f.read()
        except Exception:
            return
            
        preamp_match = re.search(r"Preamp:\s*([\-\d\.]+)\s*dB", texto)
        preamp_val = float(preamp_match.group(1)) if preamp_match else 0.0
        
        ganancias = [0.0] * 10
        qs = [1.41] * 10
        for idx in range(10):
            fc = self._fcs_bandas[idx]
            # Buscamos el filtro PK
            # Ej: Filter 1: ON PK Fc 31.5 Hz Gain 0 dB Q 1.41
            # Puede ser que use '1' o no en el numero, por lo que buscamos por Fc
            # Aunque APO usa índices, buscaremos por frecuencia para ser más robustos
            fc_str = str(fc).replace('.0', '')
            # Patron generico para encontrar la banda con la frecuencia
            patron = rf"Filter\s+\d+:\s*ON\s+PK\s+Fc\s+{fc_str}(?:\.0)?\s+Hz\s+Gain\s+([\-\d\.]+)\s*dB\s+Q\s+([\-\d\.]+)"
            match = re.search(patron, texto)
            if match:
                ganancias[idx] = float(match.group(1))
                qs[idx] = float(match.group(2))
                
        # Parse Bass (LS) and Treble (HS)
        bass_match = re.search(r"Filter\s+\d+:\s*ON\s+LS\s+Fc\s+125\s+Hz\s+Gain\s+([\-\d\.]+)\s*dB", texto)
        graves_val = float(bass_match.group(1)) if bass_match else 0.0
        
        treble_match = re.search(r"Filter\s+\d+:\s*ON\s+HS\s+Fc\s+8000\s+Hz\s+Gain\s+([\-\d\.]+)\s*dB", texto)
        agudos_val = float(treble_match.group(1)) if treble_match else 0.0

        self._actualizando_desde_archivo = True
        self.actualizandoDesdeArchivoCambiado.emit()
        
        self._preamp_externa = preamp_val
        self._ganancias_bandas = ganancias
        self._qs_bandas = qs
        self._graves_externo = graves_val
        self._agudos_externo = agudos_val
        
        self.preampExternaCambiado.emit()
        self.gananciasExternaCambiadas.emit()
        self.tonoExternoCambiado.emit()
        
        self._actualizando_desde_archivo = False
        self.actualizandoDesdeArchivoCambiado.emit()

    def cleanup(self):
        self.ejecutando = False
        self.detener_backend()

if __name__ == "__main__":
    os.environ["QSG_INFO"] = "1"
    os.environ["QT_QUICK_CONTROLS_STYLE"] = "Basic"
    
    app = QGuiApplication(sys.argv)
    engine = QQmlApplicationEngine()

    controlador = ControladorAudio()
    engine.rootContext().setContextProperty("controladorAudio", controlador)

    qml_file = os.path.join(os.path.dirname(__file__), "main.qml")
    engine.load(qml_file)

    if not engine.rootObjects():
        sys.exit(-1)
        
    resultado = app.exec()
    controlador.cleanup()
    sys.exit(resultado)
