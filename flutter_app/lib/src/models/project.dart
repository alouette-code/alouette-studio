// Mirrors core_engine::process types and ui/src/types/index.ts

class Project {
  final String id;
  final String name;
  final String command;
  final List<String> args;
  final String? cwd;
  final String? setupCommand;
  final List<String>? setupArgs;
  final bool autoRestart;
  final Map<String, String>? env;
  final double? maxCpuPercent;
  final int? maxRamMb;
  final int? port;
  final String? source;
  final String? terminalMode;
  final String? toolchain;
  final String? toolchainVersion;
  final bool enableTunnel;
  final int? maxLogLines;

  Project({
    required this.id,
    required this.name,
    required this.command,
    this.args = const [],
    this.cwd,
    this.setupCommand,
    this.setupArgs,
    this.autoRestart = false,
    this.env,
    this.maxCpuPercent,
    this.maxRamMb,
    this.port,
    this.source,
    this.terminalMode,
    this.toolchain,
    this.toolchainVersion,
    this.enableTunnel = false,
    this.maxLogLines,
  });

  factory Project.fromJson(Map<String, dynamic> json) {
    return Project(
      id: json['id'] as String,
      name: json['name'] as String,
      command: json['command'] as String,
      args: (json['args'] as List<dynamic>?)?.cast<String>() ?? [],
      cwd: json['cwd'] as String?,
      setupCommand: json['setup_command'] as String?,
      setupArgs: (json['setup_args'] as List<dynamic>?)?.cast<String>(),
      autoRestart: json['auto_restart'] as bool? ?? false,
      env: (json['env'] as Map<String, dynamic>?)?.map(
        (k, v) => MapEntry(k, v as String),
      ),
      maxCpuPercent: (json['max_cpu_percent'] as num?)?.toDouble(),
      maxRamMb: json['max_ram_mb'] as int?,
      port: json['port'] as int?,
      source: json['source'] as String?,
      terminalMode: json['terminal_mode'] as String?,
      toolchain: json['toolchain'] as String?,
      toolchainVersion: json['toolchain_version'] as String?,
      enableTunnel: json['enable_tunnel'] as bool? ?? false,
      maxLogLines: json['max_log_lines'] as int?,
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'name': name,
    'command': command,
    'args': args,
    'cwd': cwd,
    'setup_command': setupCommand,
    'setup_args': setupArgs,
    'auto_restart': autoRestart,
    'env': env,
    'max_cpu_percent': maxCpuPercent,
    'max_ram_mb': maxRamMb,
    'port': port,
    'source': source,
    'terminal_mode': terminalMode,
    'toolchain': toolchain,
    'toolchain_version': toolchainVersion,
    'enable_tunnel': enableTunnel,
    'max_log_lines': maxLogLines,
  };
}

enum ProcessStateType {
  stopped,
  setup,
  running,
  crashing,
  terminated,
  fatal;

  static ProcessStateType fromString(String s) {
    switch (s) {
      case 'Stopped': return ProcessStateType.stopped;
      case 'Setup': return ProcessStateType.setup;
      case 'Running': return ProcessStateType.running;
      case 'Crashing': return ProcessStateType.crashing;
      case 'Terminated': return ProcessStateType.terminated;
      case 'Fatal': return ProcessStateType.fatal;
      default: return ProcessStateType.stopped;
    }
  }

  String get label {
    switch (this) {
      case ProcessStateType.stopped: return 'Stopped';
      case ProcessStateType.setup: return 'Setup';
      case ProcessStateType.running: return 'Running';
      case ProcessStateType.crashing: return 'Crashing';
      case ProcessStateType.terminated: return 'Terminated';
      case ProcessStateType.fatal: return 'Fatal';
    }
  }
}

class ProcessState {
  final ProcessStateType type;
  final dynamic data;

  ProcessState({required this.type, this.data});

  factory ProcessState.fromJson(Map<String, dynamic> json) {
    return ProcessState(
      type: ProcessStateType.fromString(json['type'] as String),
      data: json['data'],
    );
  }
}

class TerminalSessionItem {
  final String id;
  final String name;

  TerminalSessionItem({required this.id, required this.name});
}

enum TerminalConnectionStatus {
  connecting, connected, error, disconnected;
}

class TerminalState {
  final String id;
  final TerminalConnectionStatus status;
  final String? errorMessage;

  TerminalState({required this.id, required this.status, this.errorMessage});
}

class ChildProcessInfo {
  final int pid;
  final String name;
  final String cmd;
  final String cwd;
  final String status;
  final double cpuPercentage;
  final int ramBytes;
  final int threadCount;
  final List<int> ports;
  final List<String> loadedModules;
  final int? parentPid;

  ChildProcessInfo({
    required this.pid,
    required this.name,
    required this.cmd,
    required this.cwd,
    required this.status,
    required this.cpuPercentage,
    required this.ramBytes,
    required this.threadCount,
    required this.ports,
    required this.loadedModules,
    this.parentPid,
  });
}

class LogLine {
  final String text;
  final String stream; // stdout, stderr, system
  final int timestamp;

  LogLine({required this.text, required this.stream, required this.timestamp});
}

class ResourceHistory {
  final Map<String, ResourceData> projects;

  ResourceHistory({this.projects = const {}});
}

class ResourceData {
  final List<double> cpu;
  final List<double> ram;
  final List<ChildProcessInfo>? processes;

  ResourceData({this.cpu = const [], this.ram = const [], this.processes});
}

class AppSettings {
  final String theme;
  final String language;
  final int maxLogLines;
  final bool autoScroll;
  final String activeLogFilter;
  final int maxHistoryPoints;
  final int maxTermOutputLength;
  final int monitorIntervalMs;
  final int fontSize;
  final int defaultLeftSidebarWidth;
  final int defaultRightSidebarWidth;
  final int defaultTabListHeight;
  final int defaultMonitorHeight;
  final int defaultConfigHeight;
  final bool desktopSingleExe;
  final bool desktopUpx;
  final String androidBuildTool;
  final String buildType;
  final String buildOutputDir;
  final String buildOutputName;
  final String buildSourceDir;
  final String buildTarget;

  AppSettings({
    this.theme = 'dark',
    this.language = 'en',
    this.maxLogLines = 2000,
    this.autoScroll = true,
    this.activeLogFilter = '',
    this.maxHistoryPoints = 30,
    this.maxTermOutputLength = 100000,
    this.monitorIntervalMs = 2000,
    this.fontSize = 13,
    this.defaultLeftSidebarWidth = 220,
    this.defaultRightSidebarWidth = 380,
    this.defaultTabListHeight = 250,
    this.defaultMonitorHeight = 250,
    this.defaultConfigHeight = 300,
    this.desktopSingleExe = false,
    this.desktopUpx = false,
    this.androidBuildTool = 'gradle',
    this.buildType = 'debug',
    this.buildOutputDir = 'build',
    this.buildOutputName = 'app',
    this.buildSourceDir = '',
    this.buildTarget = '',
  });
}

class GitDiffLine {
  final int lineNumber;
  final String changeType;
  final int deletedCount;

  GitDiffLine({required this.lineNumber, required this.changeType, required this.deletedCount});
}

class GitFileDiff {
  final List<GitDiffLine> lines;
  final bool untracked;

  GitFileDiff({required this.lines, required this.untracked});
}
