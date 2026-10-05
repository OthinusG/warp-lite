#define ProductName "Warpai Companion"
#ifndef ProductVersion
  #error ProductVersion is required
#endif
[Setup]
AppId=dev.warpai.Companion
AppName={#ProductName}
AppVersion={#ProductVersion}
VersionInfoVersion={#ProductVersion}
VersionInfoProductVersion={#ProductVersion}
DefaultDirName={userprofile}\.config\.warpai\bin
DisableDirPage=yes
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=.
OutputBaseFilename=WarpaiCompanion-windows-x64-setup
UninstallFilesDir={app}\companion-uninstall
UninstallDisplayName={#ProductName}
SetupIconFile=../../app/assets/branding/warpai.ico
Compression=lzma2
SolidCompression=yes
[Files]
Source: "../../companion-release/warpai-companion.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "../../companion-release/manifest.json"; DestDir: "{app}"; DestName: "companion-manifest.json"; Flags: ignoreversion
Source: "../../companion-release/LICENSE-AGPL"; DestDir: "{app}\companion-notices"; Flags: ignoreversion
Source: "../../companion-release/FORK_NOTICE.md"; DestDir: "{app}\companion-notices"; Flags: ignoreversion
