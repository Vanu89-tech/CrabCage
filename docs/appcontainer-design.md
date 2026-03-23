# AppContainer-Dateisystemschutz in CrabCage

## Ziel

CrabCage soll Dateipfad-Freigaben auf Windows nicht nur in der UI darstellen, sondern technisch hart erzwingen. Dafuer wird OpenClaw als AppContainer-Prozess gestartet. Ein AppContainer hat standardmaessig keinen Zugriff auf beliebige Benutzerdateien. Zugriff entsteht nur dort, wo CrabCage dem AppContainer explizit Rechte auf Dateien oder Verzeichnisse gibt.

## Schutzebenen

CrabCage trennt die Schutzwirkung in drei Ebenen:

1. Netzwerk geschuetzt
   OpenClaw erhaelt `HTTP_PROXY` und `HTTPS_PROXY` fuer den lokalen CrabCage-Proxy. Nur dort freigegebene Domains werden durchgelassen.

2. Prozesse geschuetzt
   OpenClaw und seine Kindprozesse laufen weiter in einem Windows Job Object. Nicht erlaubte Kindprozesse werden beendet.

3. Dateisystem hart geschuetzt
   OpenClaw wird als AppContainer gestartet. Nur freigegebene Pfade, die OpenClaw-Laufzeit und ein isolierter CrabCage-Arbeitsbereich erhalten passende ACL-Eintraege fuer die AppContainer-SID.

## Architektur

### 1. AppContainer-Profil

- Profilname: `CrabCage.OpenClaw`
- CrabCage versucht zuerst `DeriveAppContainerSidFromAppContainerName`.
- Falls das Profil noch nicht existiert, erstellt CrabCage es mit `CreateAppContainerProfile`.
- Als Capability wird mindestens `internetClient` gesetzt, damit OpenClaw ueber den lokalen Proxy kommunizieren kann.

### 2. Isolierter Arbeitsbereich

CrabCage legt unter `%APPDATA%\\CrabCage\\appcontainer` einen dedizierten Arbeitsbereich an:

- `home`
- `appdata`
- `localappdata`
- `temp`

Diese Verzeichnisse erhalten Schreibrechte fuer die AppContainer-SID. Vor dem Start setzt CrabCage die relevanten Umgebungsvariablen auf diese Pfade:

- `HOME`
- `USERPROFILE`
- `APPDATA`
- `LOCALAPPDATA`
- `TEMP`
- `TMP`

So schreibt OpenClaw nicht in normale Benutzerpfade, sondern in den isolierten CrabCage-Bereich.

### 3. Pfad-Freigaben

Vor dem Start fuegt CrabCage ACL-Eintraege fuer die AppContainer-SID hinzu:

- OpenClaw-Laufzeitpfade: nur Lesen/Ausfuehren
- Node-Executable: nur Lesen/Ausfuehren
- CrabCage-AppContainer-Arbeitsbereich: Lesen/Schreiben
- Benutzerdefinierte `allowedPaths`
  - `read` -> Lesen/Ausfuehren
  - `write` -> Modify

Die Rechte werden rekursiv gesetzt, damit bestehende Inhalte innerhalb freigegebener Verzeichnisse ebenfalls erreichbar sind.

Um Admin-Rechte auf geschuetzten Systempfaden zu vermeiden, kopiert CrabCage die fuer den Start benoetigte OpenClaw-Laufzeit vor dem Launch in einen CrabCage-eigenen Runtime-Ordner unter `%APPDATA%\\CrabCage\\appcontainer\\runtime`. Der AppContainer startet aus dieser gestagten Laufzeit.

## Startablauf

1. OpenClaw-Launch-Befehl aufloesen
2. AppContainer-SID ermitteln/erstellen
3. Isolierten CrabCage-Arbeitsbereich vorbereiten
4. Freigaben fuer Laufzeitpfade und `allowedPaths` setzen
5. `STARTUPINFOEX` mit `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES` aufbauen
6. OpenClaw im AppContainer starten
7. Prozess weiter dem Job Object zuweisen
8. Status an die UI zurueckmelden

## Bekannte Grenzen

- AppContainer ist strenger als das bisherige Modell. Manche Desktop-Interaktionen oder Kindprozesse koennen dadurch eingeschraenkt sein, auch wenn sie in CrabCage als erlaubte App hinterlegt sind.
- Die ACL-Freigaben gelten nur fuer die explizit erfassten Pfade. Wenn OpenClaw weitere Laufzeitdateien benoetigt, muessen diese ebenfalls freigegeben werden.
- Nicht-HTTP-Netzwerkverkehr wird weiterhin nicht ueber den Domain-Proxy erzwungen. Das Dateisystem ist damit hart geschuetzt, das Netzwerk nur fuer den aktuellen Proxy-Pfad.

## UI-Folgerung

Die Session-Anzeige unterscheidet danach klar:

- Netzwerk geschuetzt
- Prozesse geschuetzt
- Dateisystem hart geschuetzt

Nur wenn der AppContainer-Start inklusive ACL-Vorbereitung erfolgreich war, wird die dritte Schutzebene als aktiv angezeigt.
