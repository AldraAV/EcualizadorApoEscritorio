import subprocess
import time
import os

backend_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "build", "motor_audio_backend.exe"))
p = subprocess.Popen([backend_path], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)

time.sleep(1) # wait for init
p.stdin.write("PREAMP 15.5\n")
p.stdin.flush()
time.sleep(1) # wait for write
p.terminate()
