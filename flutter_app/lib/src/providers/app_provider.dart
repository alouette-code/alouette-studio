import 'package:flutter/material.dart';
import 'package:collection/collection.dart';
import '../models/project.dart';
import '../services/rust_bridge_service.dart';

/// Central application state provider mirroring all state from React App.tsx
class AppProvider extends ChangeNotifier {
  final RustBridgeService _bridge;

  RustBridgeService get bridge => _bridge;

  AppProvider(this._bridge);

  // ── Theme ──
  bool _isDark = true;
  bool get isDark => _isDark;
  void toggleTheme() {
    _isDark = !_isDark;
    notifyListeners();
  }
  void setTheme(bool dark) {
    if (_isDark != dark) {
      _isDark = dark;
      notifyListeners();
    }
  }

  // ── Projects ──
  List<Project> _projects = [];
  List<Project> get projects => _projects;

  String _activeProjectId = '';
  String get activeProjectId => _activeProjectId;
  set activeProjectId(String id) {
    _activeProjectId = id;
    notifyListeners();
  }

  Project? get activeProject =>
      _projects.firstWhereOrNull((p) => p.id == _activeProjectId);

  Map<String, ProcessState> _projectStates = {};
  Map<String, ProcessState> get projectStates => _projectStates;

  Map<String, List<LogLine>> _projectLogs = {};
  Map<String, List<LogLine>> get projectLogs => _projectLogs;

  ProcessState get activeState => _projectStates[_activeProjectId] ??
      ProcessState(type: ProcessStateType.stopped);

  Future<void> loadProjects() async {
    try {
      _projects = await _bridge.getProjects();
      notifyListeners();
    } catch (e) {
      debugPrint('loadProjects error: $e');
    }
  }

  Future<void> handleStartProject(String projectId) async {
    try {
      await _bridge.startProjectProcess(projectId);
      notifyListeners();
    } catch (e) {
      debugPrint('startProject error: $e');
    }
  }

  Future<void> handleStopProject(String projectId) async {
    try {
      await _bridge.stopProjectProcess(projectId);
      notifyListeners();
    } catch (e) {
      debugPrint('stopProject error: $e');
    }
  }

  Future<void> handleAddProject() async {
    // Will be implemented with form data
    notifyListeners();
  }

  Future<void> handleDeleteProject(String projectId) async {
    try {
      await _bridge.deregisterProject(projectId);
      await loadProjects();
    } catch (e) {
      debugPrint('deleteProject error: $e');
    }
  }

  // ── Layout ──
  double _leftSidebarWidth = 220;
  double get leftSidebarWidth => _leftSidebarWidth;
  set leftSidebarWidth(double w) {
    _leftSidebarWidth = w.clamp(160, 450);
    notifyListeners();
  }

  double _rightSidebarWidth = 380;
  double get rightSidebarWidth => _rightSidebarWidth;
  set rightSidebarWidth(double w) {
    _rightSidebarWidth = w.clamp(280, 600);
    notifyListeners();
  }

  double _tabListHeight = 250;
  double get tabListHeight => _tabListHeight;
  set tabListHeight(double h) {
    _tabListHeight = h.clamp(80, 400);
    notifyListeners();
  }

  double _monitorHeight = 250;
  double get monitorHeight => _monitorHeight;
  set monitorHeight(double h) {
    _monitorHeight = h.clamp(100, 500);
    notifyListeners();
  }

  double _configHeight = 300;
  double get configHeight => _configHeight;
  set configHeight(double h) {
    _configHeight = h.clamp(120, 450);
    notifyListeners();
  }

  bool _isLeftSidebarOpen = true;
  bool get isLeftSidebarOpen => _isLeftSidebarOpen;
  void toggleLeftSidebar() {
    _isLeftSidebarOpen = !_isLeftSidebarOpen;
    notifyListeners();
  }

  bool _isBottomPanelOpen = true;
  bool get isBottomPanelOpen => _isBottomPanelOpen;
  void toggleBottomPanel() {
    _isBottomPanelOpen = !_isBottomPanelOpen;
    notifyListeners();
  }

  bool _isRightSidebarOpen = true;
  bool get isRightSidebarOpen => _isRightSidebarOpen;
  void toggleRightSidebar() {
    _isRightSidebarOpen = !_isRightSidebarOpen;
    notifyListeners();
  }

  bool _isAiViewActive = false;
  bool get isAiViewActive => _isAiViewActive;
  void setAiViewActive(bool v) {
    _isAiViewActive = v;
    if (v) _isGitViewActive = false;
    notifyListeners();
  }

  bool _isGitViewActive = false;
  bool get isGitViewActive => _isGitViewActive;
  void setGitViewActive(bool v) {
    _isGitViewActive = v;
    if (v) _isAiViewActive = false;
    notifyListeners();
  }

  // ── Editor Panes ──
  List<EditorPane> _panes = [EditorPane()];
  List<EditorPane> get panes => _panes;

  int _activePaneIndex = 0;
  int get activePaneIndex => _activePaneIndex;

  void setActivePaneIndex(int idx) {
    if (idx >= 0 && idx < _panes.length) {
      _activePaneIndex = idx;
      notifyListeners();
    }
  }

  String? get activePaneFilePath => _panes[_activePaneIndex].openFilePath;

  void handleFileOpen(String path, {int? line}) {
    final normalized = path.replaceAll('\\', '/');
    final pane = _panes[_activePaneIndex];
    final files = List<String>.from(pane.openFiles);
    if (!files.contains(normalized)) {
      files.add(normalized);
    }
    _panes[_activePaneIndex] = EditorPane(
      openFiles: files,
      openFilePath: normalized,
    );
    notifyListeners();
  }

  void handleFileClose(int paneIdx, String path) {
    final normalized = path.replaceAll('\\', '/');
    final pane = _panes[paneIdx];
    final files = List<String>.from(pane.openFiles);
    files.remove(normalized);
    _panes[paneIdx] = EditorPane(
      openFiles: files,
      openFilePath: files.isNotEmpty ? files.last : null,
    );
    notifyListeners();
  }

  void handleCloseAllTabs() {
    _panes = [EditorPane()];
    _activePaneIndex = 0;
    notifyListeners();
  }

  void handleSplit() {
    if (_panes.length >= 3) return;
    final current = _panes[_activePaneIndex];
    _panes.add(EditorPane(
      openFiles: current.openFilePath != null ? [current.openFilePath!] : [],
      openFilePath: current.openFilePath,
    ));
    _activePaneIndex = _panes.length - 1;
    notifyListeners();
  }

  void handleClosePane(int paneIdx) {
    if (_panes.length <= 1) return;
    _panes.removeAt(paneIdx);
    _activePaneIndex = 0;
    notifyListeners();
  }

  void handleDrop(int sourceIdx, int targetIdx, String path) {
    if (sourceIdx == targetIdx) return;
    final panes = List<EditorPane>.from(_panes);
    final source = panes[sourceIdx];
    final target = panes[targetIdx];

    final sourceFiles = List<String>.from(source.openFiles);
    sourceFiles.remove(path);
    panes[sourceIdx] = EditorPane(
      openFiles: sourceFiles,
      openFilePath: source.openFilePath == path
          ? (sourceFiles.isNotEmpty ? sourceFiles.last : null)
          : source.openFilePath,
    );

    final targetFiles = List<String>.from(target.openFiles);
    if (!targetFiles.contains(path)) targetFiles.add(path);
    panes[targetIdx] = EditorPane(openFiles: targetFiles, openFilePath: path);

    _panes = panes;
    _activePaneIndex = targetIdx;
    notifyListeners();
  }

  // ── File Content ──
  Map<String, String> _filesContent = {};
  Map<String, String> get filesContent => _filesContent;
  void setFileContent(String path, String content) {
    _filesContent[path] = content;
    notifyListeners();
  }

  // ── Terminal ──
  List<TerminalSessionItem> _terminals = [];
  List<TerminalSessionItem> get terminals => _terminals;

  String? _activeTerminalId;
  String? get activeTerminalId => _activeTerminalId;
  void setActiveTerminalId(String? id) {
    _activeTerminalId = id;
    notifyListeners();
  }

  // ── Resources ──
  ResourceHistory _resourceHistory = ResourceHistory();
  ResourceHistory get resourceHistory => _resourceHistory;

  // ── Search ──
  String _searchQuery = '';
  String get searchQuery => _searchQuery;
  void setSearchQuery(String q) {
    _searchQuery = q;
    notifyListeners();
  }

  List<Project> get filteredProjects {
    if (_searchQuery.isEmpty) return _projects;
    final q = _searchQuery.toLowerCase();
    return _projects.where((p) =>
      p.name.toLowerCase().contains(q) ||
      p.command.toLowerCase().contains(q) ||
      p.args.join(' ').toLowerCase().contains(q)
    ).toList();
  }

  // ── Toast ──
  ToastMessage? _toast;
  ToastMessage? get toast => _toast;
  void showToast(String message, {String type = 'info'}) {
    _toast = ToastMessage(message, type);
    notifyListeners();
    Future.delayed(const Duration(seconds: 3), () {
      _toast = null;
      notifyListeners();
    });
  }
  void dismissToast() {
    _toast = null;
    notifyListeners();
  }

  // ── Confirm Modal ──
  ConfirmDialog? _confirmModal;
  ConfirmDialog? get confirmModal => _confirmModal;
  void showConfirm(String message, VoidCallback onConfirm, {VoidCallback? onCancel}) {
    _confirmModal = ConfirmDialog(message, onConfirm, onCancel);
    notifyListeners();
  }
  void dismissConfirm() {
    _confirmModal = null;
    notifyListeners();
  }

  // ── File Prompt ──
  FilePrompt? _filePrompt;
  FilePrompt? get filePrompt => _filePrompt;
  void showFilePrompt(String title, String placeholder, String defaultValue,
      Function(String) onOk) {
    _filePrompt = FilePrompt(title, placeholder, defaultValue, onOk);
    notifyListeners();
  }
  void dismissFilePrompt() {
    _filePrompt = null;
    notifyListeners();
  }
}

class EditorPane {
  final List<String> openFiles;
  final String? openFilePath;

  EditorPane({
    this.openFiles = const [],
    this.openFilePath,
  });
}

class ToastMessage {
  final String message;
  final String type; // success, error, info
  ToastMessage(this.message, this.type);
}

class ConfirmDialog {
  final String message;
  final VoidCallback onConfirm;
  final VoidCallback? onCancel;
  ConfirmDialog(this.message, this.onConfirm, this.onCancel);
}

class FilePrompt {
  final String title;
  final String placeholder;
  final String defaultValue;
  final Function(String) onOk;
  FilePrompt(this.title, this.placeholder, this.defaultValue, this.onOk);
}
