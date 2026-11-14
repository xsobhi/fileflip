; FileFlip setup (Inno Setup 6). Built by CI: iscc /DVersion=x.y.z installer\fileflip.iss
; The setup just unpacks the app and bundled FFmpeg to a temp folder and runs
; "fileflip.exe install", which copies everything to %LOCALAPPDATA%\Programs\FileFlip,
; registers the right-click menu and adds the Apps & features uninstall entry.

#ifndef Version
  #define Version "0.0.0"
#endif

[Setup]
AppName=FileFlip
AppVersion={#Version}
AppPublisher=xsobhi
AppPublisherURL=https://github.com/xsobhi/fileflip
PrivilegesRequired=lowest
CreateAppDir=no
Uninstallable=no
DisableProgramGroupPage=yes
DisableReadyPage=yes
WizardStyle=modern
SetupIconFile=..\assets\fileflip.ico
OutputDir=..\dist
OutputBaseFilename=FileFlip-Setup-{#Version}
Compression=lzma2/ultra64
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Files]
Source: "..\dist\FileFlip\*"; DestDir: "{tmp}\FileFlip"; Flags: recursesubdirs ignoreversion

[Run]
Filename: "{tmp}\FileFlip\fileflip.exe"; Parameters: "install"; StatusMsg: "Installing FileFlip…"; Flags: waituntilterminated
