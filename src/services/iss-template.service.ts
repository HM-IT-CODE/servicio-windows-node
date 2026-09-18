import crypto from 'crypto'
import { WinsvcConfig, InstallerConfig, InstallerPrompt } from '../models/winsvc-config.model'

/**
 * Builds an Inno Setup script (.iss) from winsvc.config.json.
 *
 * The generated wizard: checks Node is present, asks for whatever the config
 * declares in `prompts`, writes them to a .env, runs the verify script, and
 * only then registers the service. If verification fails it stops short of
 * registering — a bad .env with the service already installed produces a
 * crash-loop that looks like a broken program from the outside.
 */
export class IssTemplateService {

  build(
    config: WinsvcConfig,
    installer: InstallerConfig,
    corePath: string,
    bundledNode: string | null = null,
  ): string {
    const prompts = installer.prompts ?? []
    const groups  = this.groupPrompts(prompts)

    return [
      this.header(config, installer),
      this.setup(config, installer),
      this.languages(installer),
      this.files(installer, corePath, bundledNode),
      this.dirs(config),
      this.uninstall(config),
      this.code(config, installer, groups, bundledNode !== null),
    ].join('\n')
  }

  /** Groups prompts into wizard pages, preserving declaration order. */
  private groupPrompts(prompts: InstallerPrompt[]): Map<string, InstallerPrompt[]> {
    const groups = new Map<string, InstallerPrompt[]>()

    for (const prompt of prompts) {
      const name = prompt.group ?? 'Configuración'
      const list = groups.get(name) ?? []
      list.push(prompt)
      groups.set(name, list)
    }
    return groups
  }

  /**
   * A stable GUID derived from the service name, so rebuilding the installer
   * keeps upgrading the same installation instead of creating a second entry.
   */
  private appId(installer: InstallerConfig, serviceName: string): string {
    if (installer.appId) return installer.appId

    const hash = crypto.createHash('sha256').update(`node-winsvc:${serviceName}`).digest('hex')
    return [
      hash.slice(0, 8),  hash.slice(8, 12), hash.slice(12, 16),
      hash.slice(16, 20), hash.slice(20, 32),
    ].join('-').toUpperCase()
  }

  private header(config: WinsvcConfig, installer: InstallerConfig): string {
    return `; ============================================================================
;  Instalador de ${installer.appName}
;  GENERADO por Aruna a partir de winsvc.config.json — no editar a mano:
;  se sobrescribe en la siguiente ejecución de \`aruna installer\`.
;
;  Servicio: ${config.name}
; ============================================================================
`
  }

  private setup(config: WinsvcConfig, installer: InstallerConfig): string {
    const dirName = installer.defaultDirName ?? installer.appName

    return `[Setup]
AppId={{${this.appId(installer, config.name)}}
AppName=${installer.appName}
AppVersion=${installer.version}
AppPublisher=${installer.publisher}
DefaultDirName={autopf}\\${dirName}
DefaultGroupName=${installer.appName}
; Relativo al propio .iss: Inno lo resuelve contra el script, no contra el
; proyecto. Poner aquí la ruta del config anidaría instalador\instalador\.
OutputDir=.
OutputBaseFilename=instalar-${config.name}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
; Registrar un servicio exige elevación: la pedimos de entrada.
PrivilegesRequired=admin
ArchitecturesInstallIn64BitMode=x64compatible
DisableProgramGroupPage=yes
UninstallDisplayName=${installer.appName}
`
  }

  private languages(installer: InstallerConfig): string {
    const file = installer.language === 'en' ? 'Default.isl' : 'Languages\\Spanish.isl'
    return `[Languages]\nName: "app"; MessagesFile: "compiler:${file}"\n`
  }

  private files(installer: InstallerConfig, corePath: string, bundledNode: string | null): string {
    const lines = ['[Files]']

    for (const item of installer.include ?? ['src', 'package.json']) {
      const clean = item.replace(/\//g, '\\').replace(/\\$/, '')
      const isDir = !clean.includes('.') || clean.endsWith('*')

      lines.push(isDir
        ? `Source: "{#Origen}\\${clean}\\*"; DestDir: "{app}\\${clean}"; Flags: ignoreversion recursesubdirs createallsubdirs`
        : `Source: "{#Origen}\\${clean}"; DestDir: "{app}"; Flags: ignoreversion`)
    }

    lines.push(`; El núcleo que habla con el Service Control Manager`)
    lines.push(`Source: "${corePath}"; DestDir: "{app}\\bin"; Flags: ignoreversion`)

    if (bundledNode) {
      // node_finder.rs lo prefiere sobre el Node del sistema, así el servidor
      // de destino no necesita tener Node instalado.
      lines.push(`; Node empaquetado: el servidor de destino no necesita Node`)
      lines.push(`Source: "{#Origen}\\${bundledNode.replace(/\//g, '\\')}"; DestDir: "{app}\\vendor"; Flags: ignoreversion`)
    }

    return lines.join('\n') + '\n'
  }

  private dirs(config: WinsvcConfig): string {
    const logDir = config.logFile.replace(/\//g, '\\').split('\\').slice(0, -1).join('\\') || 'logs'
    return `[Dirs]\nName: "{app}\\${logDir}"\n`
  }

  private uninstall(config: WinsvcConfig): string {
    return `[UninstallRun]
; Parar y quitar el servicio ANTES de borrar archivos, o Windows deja el exe bloqueado.
Filename: "{app}\\bin\\node-winsvc-core.exe"; Parameters: "stop --name ${config.name}"; Flags: runhidden; RunOnceId: "PararServicio"
Filename: "{app}\\bin\\node-winsvc-core.exe"; Parameters: "uninstall --name ${config.name}"; Flags: runhidden; RunOnceId: "QuitarServicio"

[UninstallDelete]
Type: filesandordirs; Name: "{app}\\logs"
Type: files; Name: "{app}\\.env"
`
  }

  private code(
    config: WinsvcConfig,
    installer: InstallerConfig,
    groups: Map<string, InstallerPrompt[]>,
    traeNode: boolean,
  ): string {
    const pages = [...groups.entries()]

    return `
; ============================================================================
[Code]

var
${pages.map((_, i) => `  Pagina${i}: TInputQueryWizardPage;`).join('\n') || '  SinPaginas: Integer;'}

procedure InitializeWizard();
begin
${pages.map(([titulo, prompts], i) => this.pageCode(titulo, prompts, i, pages)).join('\n')}
end;

function NodeInstalado(): Boolean;
var Codigo: Integer;
begin
  Result := Exec('cmd.exe', '/c where node', '', SW_HIDE, ewWaitUntilTerminated, Codigo) and (Codigo = 0);
end;

function InitializeSetup(): Boolean;
begin
  Result := True;
${traeNode
  ? "  { Este instalador trae su propio node.exe: no exige nada al servidor. }"
  : `  if not NodeInstalado() then
  begin
    MsgBox('No se encontró node.exe en el PATH.' + #13#10 + #13#10 +
           'Instale Node.js 18 o superior y vuelva a ejecutar este instalador.',
           mbCriticalError, MB_OK);
    Result := False;
  end;`}
end;

procedure EscribirEnv();
var
  Lineas: TArrayOfString;
begin
  SetArrayLength(Lineas, ${this.envLineCount(config, groups)});
${this.envCode(config, groups)}
  SaveStringsToFile(ExpandConstant('{app}\\.env'), Lineas, False);
end;

${this.verifyCode(installer)}

function RegistrarServicio(): Boolean;
var
  Codigo: Integer;
  Params: String;
begin
  { Pararlo antes de borrarlo: DeleteService solo lo MARCA, el proceso sigue
    vivo reteniendo el puerto y el servicio nuevo arrancaría contra él. }
  Exec(ExpandConstant('{app}\\bin\\node-winsvc-core.exe'), 'stop --name ${config.name}', '', SW_HIDE, ewWaitUntilTerminated, Codigo);
  Sleep(2000);
  Exec(ExpandConstant('{app}\\bin\\node-winsvc-core.exe'), 'uninstall --name ${config.name}', '', SW_HIDE, ewWaitUntilTerminated, Codigo);
  Sleep(1000);

  Params := 'install' +
    ' --name ${config.name}' +
    ' --display "${config.displayName}"' +
    ' --description "${config.description.replace(/"/g, "''")}"' +
    ' --script "'      + ExpandConstant('{app}\\${config.script.replace(/\//g, '\\')}') + '"' +
    ' --node-args "${config.nodeArgs.join(' ')}"' +
    ' --env "{}"' +
    ' --working-dir "' + ExpandConstant('{app}') + '"' +
    ' --log-file "'    + ExpandConstant('{app}\\${config.logFile.replace(/\//g, '\\')}') + '"' +
    ' --start-type ${config.startType}' +
    ' --auto-restart ${config.autoRestart}';

  Result := Exec(ExpandConstant('{app}\\bin\\node-winsvc-core.exe'), Params,
                 ExpandConstant('{app}'), SW_HIDE, ewWaitUntilTerminated, Codigo) and (Codigo = 0);
end;

function ArrancarServicio(): Boolean;
var Codigo: Integer;
begin
  Result := Exec(ExpandConstant('{app}\\bin\\node-winsvc-core.exe'), 'start --name ${config.name}',
                 '', SW_HIDE, ewWaitUntilTerminated, Codigo) and (Codigo = 0);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep <> ssPostInstall then Exit;

  WizardForm.StatusLabel.Caption := 'Escribiendo la configuración...';
  EscribirEnv();

  if not Verificar() then
  begin
    MsgBox('La comprobación de la configuración falló.' + #13#10 + #13#10 +
           'Los archivos quedaron instalados, pero el servicio NO se registró.' + #13#10 +
           'Corrija el archivo .env en la carpeta de instalación y vuelva a' + #13#10 +
           'ejecutar este instalador con los datos correctos.', mbError, MB_OK);
    Exit;
  end;

  WizardForm.StatusLabel.Caption := 'Registrando el servicio de Windows...';
  if not RegistrarServicio() then
  begin
    MsgBox('No se pudo registrar el servicio.' + #13#10 +
           'Compruebe que está ejecutando el instalador como Administrador.', mbError, MB_OK);
    Exit;
  end;

  WizardForm.StatusLabel.Caption := 'Arrancando el servicio...';
  if not ArrancarServicio() then
  begin
    MsgBox('El servicio quedó registrado pero no arrancó.' + #13#10 +
           'Revise el log en la carpeta de instalación.', mbError, MB_OK);
    Exit;
  end;

  MsgBox('Listo.' + #13#10 + #13#10 +
         'El servicio "${config.displayName}" está corriendo y arranca solo' + #13#10 +
         'cada vez que se encienda el equipo.', mbInformation, MB_OK);
end;
`
  }

  private pageCode(
    titulo: string,
    prompts: InstallerPrompt[],
    index: number,
    pages: [string, InstallerPrompt[]][],
  ): string {
    const anterior = index === 0 ? 'wpSelectDir' : `Pagina${index - 1}.ID`
    const campos = prompts.map(p =>
      `  Pagina${index}.Add('${p.label}:', ${p.secret ? 'True' : 'False'});`).join('\n')
    const valores = prompts.map((p, i) =>
      `  Pagina${index}.Values[${i}] := '${(p.default ?? '').replace(/'/g, "''")}';`).join('\n')

    void pages
    return `  Pagina${index} := CreateInputQueryPage(${anterior},
    '${titulo}', '${titulo}',
    'Indique los datos necesarios para que el servicio funcione.');
${campos}
${valores}
`
  }

  private envLineCount(config: WinsvcConfig, groups: Map<string, InstallerPrompt[]>): number {
    const prompts = [...groups.values()].flat().length
    return prompts + Object.keys(config.env).length + 1
  }

  private envCode(config: WinsvcConfig, groups: Map<string, InstallerPrompt[]>): string {
    const lines: string[] = [`  Lineas[0] := '# Generado por el instalador de Aruna';`]
    let i = 1

    for (const [key, value] of Object.entries(config.env)) {
      lines.push(`  Lineas[${i}] := '${key}=${value}';`)
      i += 1
    }

    let page = 0
    for (const prompts of groups.values()) {
      prompts.forEach((prompt, field) => {
        lines.push(`  Lineas[${i}] := '${prompt.key}=' + Trim(Pagina${page}.Values[${field}]);`)
        i += 1
      })
      page += 1
    }

    return lines.join('\n')
  }

  /** Without a verify script the wizard proceeds; with one, it must pass. */
  private verifyCode(installer: InstallerConfig): string {
    if (!installer.verifyScript) {
      return `function Verificar(): Boolean;\nbegin\n  Result := True;\nend;`
    }

    const script = installer.verifyScript.replace(/\//g, '\\')
    return `function Verificar(): Boolean;
var Codigo: Integer;
begin
  WizardForm.StatusLabel.Caption := 'Comprobando la configuración...';
  Result := Exec('cmd.exe', '/c node "' + ExpandConstant('{app}\\${script}') + '"',
                 ExpandConstant('{app}'), SW_HIDE, ewWaitUntilTerminated, Codigo) and (Codigo = 0);
end;`
  }
}
