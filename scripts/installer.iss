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
Name: "{app}\logs"

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
[Code]
var
  LogPage: TInputDirWizardPage;

function InitializeSetup(): Boolean;
begin
  LogPage := CreateInputDirWizardPage(
    wpSelectDir,
    'ログフォルダの選択',
    'winletmeknow のログファイル (daemon.log) を保存するフォルダを選択してください。',
    '',
    ExpandConstant('{app}\logs'),
    False
  );
  Result := True;
end;

function CurStepChanged(CurStep: TSetupStep): Boolean;
var
  LogDir: String;
  ConfigContent: String;
begin
  Result := True;

  if CurStep = ssPostInstall then
  begin
    LogDir := LogPage.Values[0];

    ; config.json を生成（ログフォルダ選択結果を log_dir に設定）
    ConfigContent :=
      '{' + #13#10 +
      '  "port": 37821,' + #13#10 +
      '  "aumid": "Mango.winLetmeknow",' + #13#10 +
      '  "default_preset": "claude",' + #13#10 +
      '  "log_dir": "' + LogDir + '",' + #13#10 +
      '  "presets": {' + #13#10 +
      '    "claude": { "name": "Claude Code", "icon": "assets/claude.png", "color": [128, 90, 180] },' + #13#10 +
      '    "codex":  { "name": "Codex CLI",   "icon": "assets/codex.png",  "color": [30, 140, 170] },' + #13#10 +
      '    "aider":  { "name": "Aider",       "icon": "assets/aider.png",  "color": [90, 150, 110] }' + #13#10 +
      '  }' + #13#10 +
      '}' + #13#10;

    SaveStringToFile(ExpandConstant('{app}\config.json'), ConfigContent, False);
  end;
end;
