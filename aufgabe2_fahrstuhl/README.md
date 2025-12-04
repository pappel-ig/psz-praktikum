# Projekt-Setup und Ausführung

## Docker-Compose Starten

Öffnen Sie ein Terminal und navigieren Sie zum Projektverzeichnis. Starten Sie die Docker-Compose-Konfiguration mit folgendem Befehl:

```bash
docker compose up
```

## Node-RED Benutzeroberfläche Aufrufen

Nachdem die Docker-Container gestartet sind, öffnen Sie die Node-RED Benutzeroberfläche in Ihrem Webbrowser:

```
http://localhost:1880/ui
```

## Fahrstuhl-Anwendung Starten

Führen Sie die folgenden Befehle aus, um die Fahrstuhl-Anwendung zu kompilieren und zu starten:

```bash
cargo build
cargo run
```

## Hinweise

* Stellen Sie sicher, dass Docker und Cargo auf Ihrem System installiert sind.
* Überprüfen Sie, dass alle Abhängigkeiten korrekt eingerichtet sind, bevor Sie die Befehle ausführen.