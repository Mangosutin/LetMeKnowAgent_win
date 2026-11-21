; winletmeknow インストーラースクリプト (Inno Setup)
; ビルド: Inno Setup Compiler でこのファイルを開いてコンパイル
;   https://jrsoftware.org/isinfo.php

#define MyAppName "winletmeknow"
#define MyAppVersion "0.1.0"
#define MyAppPublisher "Mango"
#define MyAppExeName "winletmeknow.exe"
#define MyAppUrl "https://github.com/Mangosutin/LetMeKnowAgent_win"

[Setup]
AppId={{2A3F1B4C-5D6E-7F8A-9B0C-1D2E3F4A5B6C}}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppUrl}
DefaultDirName={pf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
LicenseFile=
OutputDir=installer
OutputBaseFilename=winletmeknow_setup_{#MyAppVersion}
Compression=lzma
SolidCompression=yes
PrivilegesRequired=admin
ChangesAssociations=yes

; ------------------------------------------------------------
[Languages]
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

; ------------------------------------------------------------
[Tasks]
Name: "desktopicon"; Description: "デスクトップにショートカットを作成"; GroupDescription: "追加ショートカット"; Flags: unchecked
Name: "quicklaunchicon"; Description: "タスクバーにショートカットを作成"; GroupDescription: "追加ショートカット"; Flags: unchecked

; ------------------------------------------------------------
[Dirs]
Name: "{app}\assets"

; ------------------------------------------------------------
[Files]
Source: "target\release\winletmeknow.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "target\release\assets\*"; DestDir: "{app}\assets"; Flags: ignoreversion recursesubdirs

; ------------------------------------------------------------
[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{commondesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

; ------------------------------------------------------------
[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{#MyAppName} を起動"; Flags: nowait postinstall skipifsilent

; ------------------------------------------------------------
