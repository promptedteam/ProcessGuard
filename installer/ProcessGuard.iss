#define AppName "Process Guard"
#define AppVersion "1.0.2"
#define AppExeName "ProcessGuard.exe"
#define AppId "{{93857765-1B62-47EE-9220-9B94D6A7D613}"

[Setup]
AppId={#AppId}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppName}
AppComments=Native Windows process monitoring, safety guidance, and Sentinel AI chat.
DefaultDirName={localappdata}\Programs\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
AllowNoIcons=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.17763
WizardStyle=modern dynamic
DisableWelcomePage=no
InfoBeforeFile=INSTALLER_GUIDE.txt
InfoAfterFile=GETTING_STARTED.txt
SetupIconFile=..\ProcessGuard.ico
UninstallDisplayName={#AppName}
UninstallDisplayIcon={app}\{#AppExeName}
UninstallFilesDir={app}\Uninstall
Uninstallable=yes
CreateUninstallRegKey=yes
CloseApplications=force
CloseApplicationsFilter={#AppExeName}
RestartApplications=no
UsePreviousAppDir=yes
UsePreviousTasks=yes
SetupLogging=yes
Compression=lzma2/ultra64
SolidCompression=yes
LZMAUseSeparateProcess=yes
OutputDir=..\dist
OutputBaseFilename=ProcessGuard-Setup-{#AppVersion}
VersionInfoVersion={#AppVersion}.0
VersionInfoProductVersion={#AppVersion}.0
VersionInfoProductName={#AppName}
VersionInfoDescription={#AppName} Setup
VersionInfoCompany={#AppName}
VersionInfoCopyright=Copyright (c) 2026 Process Guard

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Shortcuts:"; Flags: unchecked
Name: "startup"; Description: "Start Process Guard when I sign in to Windows"; GroupDescription: "Startup:"; Flags: unchecked

[Files]
Source: "..\target\release\process_guard.exe"; DestDir: "{app}"; DestName: "{#AppExeName}"; Flags: ignoreversion
Source: "..\ProcessGuard.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "ProcessGuard.installed"; DestDir: "{app}"; Flags: ignoreversion
Source: "GETTING_STARTED.txt"; DestDir: "{app}"; Flags: ignoreversion isreadme
Source: "INSTALLER_GUIDE.txt"; DestDir: "{app}"; DestName: "Installation and Safety Guide.txt"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"; WorkingDir: "{app}"; IconFilename: "{app}\{#AppExeName}"; Comment: "Open Process Guard"
Name: "{group}\Getting Started"; Filename: "{app}\GETTING_STARTED.txt"; Comment: "Read the Process Guard quick-start guide"
Name: "{group}\Uninstall {#AppName}"; Filename: "{uninstallexe}"; Comment: "Remove Process Guard"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; WorkingDir: "{app}"; IconFilename: "{app}\{#AppExeName}"; Tasks: desktopicon
Name: "{userstartup}\{#AppName}"; Filename: "{app}\{#AppExeName}"; WorkingDir: "{app}"; IconFilename: "{app}\{#AppExeName}"; Tasks: startup

[Run]
Filename: "{app}\{#AppExeName}"; Description: "Launch {#AppName}"; WorkingDir: "{app}"; Flags: nowait postinstall skipifsilent

[Code]
procedure InitializeWizard;
begin
  WizardForm.WelcomeLabel1.Caption := 'Install Process Guard {#AppVersion}';
  WizardForm.WelcomeLabel2.Caption :=
    'This wizard installs the native Process Guard desktop application.' + #13#10 + #13#10 +
    'You will review its monitoring features, safety limits, Sentinel AI provider choices, protected credential storage, local data, optional shortcuts, and uninstall behavior before installation begins.';
end;

procedure CurPageChanged(CurPageID: Integer);
begin
  if CurPageID = wpSelectTasks then
    WizardForm.PageDescriptionLabel.Caption :=
      'Choose optional shortcuts. Neither option is required to use Process Guard.';

  if CurPageID = wpReady then
    WizardForm.PageDescriptionLabel.Caption :=
      'Review the choices below, then select Install. Existing installations are upgraded in place.';

  if CurPageID = wpFinished then
    WizardForm.FinishedLabel.Caption :=
      'Process Guard has been installed. Launch it now or open it later from the Start Menu. Use its Admin control only when a process action requires elevation.';
end;

procedure InitializeUninstallProgressForm;
begin
  UninstallProgressForm.Caption := 'Uninstall Process Guard';
  UninstallProgressForm.PageNameLabel.Caption := 'Removing Process Guard';
  UninstallProgressForm.StatusLabel.Caption :=
    'Closing the application and removing installed files and shortcuts...';
end;

function HasUninstallParam(const Value: String): Boolean;
var
  I: Integer;
begin
  Result := False;
  for I := 1 to ParamCount do
  begin
    if CompareText(ParamStr(I), Value) = 0 then
    begin
      Result := True;
      Exit;
    end;
  end;
end;

procedure DeleteSentinelCredential(const ProviderId: String);
var
  ResultCode: Integer;
begin
  Exec(
    ExpandConstant('{sys}\cmdkey.exe'),
    '/delete:ProcessGuard/Sentinel/' + ProviderId,
    '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  RemoveData: Boolean;
begin
  if CurUninstallStep = usUninstall then
  begin
    if UninstallSilent then
      RemoveData := HasUninstallParam('/REMOVEUSERDATA')
    else
      RemoveData := MsgBox(
        'Remove all locally generated Process Guard data too?' + #13#10 + #13#10 +
        'YES removes Sentinel chats and pins, watchlists, alert and automation rules, snapshots, exported reports, and all Sentinel API credentials stored for this Windows account.' + #13#10 + #13#10 +
        'NO preserves those files in the installation folder so a later reinstall can use them.',
        mbConfirmation, MB_YESNO) = IDYES;

    if RemoveData then
    begin
      DeleteFile(ExpandConstant('{app}\ProcessGuard_Sentinel_History.txt'));
      DeleteFile(ExpandConstant('{app}\ProcessGuard_Settings.txt'));
      DeleteFile(ExpandConstant('{app}\ProcessGuard_Snapshot.txt'));
      DeleteFile(ExpandConstant('{app}\ProcessGuard_Report.txt'));
      DeleteSentinelCredential('openai');
      DeleteSentinelCredential('anthropic');
      DeleteSentinelCredential('gemini-api');
      DeleteSentinelCredential('xai');
      DeleteSentinelCredential('groq');
      DeleteSentinelCredential('mistral');
      DeleteSentinelCredential('cohere');
      DeleteSentinelCredential('deepseek');
      DeleteSentinelCredential('openrouter');
      DeleteSentinelCredential('together');
    end;
  end;
end;
