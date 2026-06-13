import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/app_provider.dart';
import '../theme/app_theme.dart';

/// Code editor with file tabs, line numbers, a monospace editing area,
/// git diff gutter indicators, dirty-state save button, and an inline
/// code RAG search panel.
class CodeEditorWidget extends StatefulWidget {
  final String? filePath;
  final String content;
  final ValueChanged<String>? onChange;

  const CodeEditorWidget({
    super.key,
    this.filePath,
    this.content = '',
    this.onChange,
  });

  @override
  State<CodeEditorWidget> createState() => _CodeEditorWidgetState();
}

class _CodeEditorWidgetState extends State<CodeEditorWidget> {
  late TextEditingController _controller;
  final ScrollController _scrollController = ScrollController();
  final ScrollController _lineNumberScrollController = ScrollController();
  bool _isDirty = false;
  bool _isLoading = false;
  String? _errorMessage;

  // RAG search panel state
  bool _showRagPanel = false;
  final TextEditingController _ragSearchController = TextEditingController();
  String _ragSearchQuery = '';
  final List<_RagSearchMatch> _ragMatches = [];
  int _activeMatchIndex = 0;

  // Drag-drop split state
  int? _dragOverPaneIdx;

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController(text: widget.content);
    _controller.addListener(_onTextChanged);

    _scrollController.addListener(_syncLineNumberScroll);
  }

  @override
  void didUpdateWidget(CodeEditorWidget oldWidget) {
    super.didUpdateWidget(oldWidget);
    // Sync content from provider if the file changed externally
    if (widget.filePath != oldWidget.filePath) {
      _isDirty = false;
      _errorMessage = null;
    }
    if (widget.content != _controller.text && !_isDirty) {
      _controller.text = widget.content;
    }
  }

  @override
  void dispose() {
    _scrollController.removeListener(_syncLineNumberScroll);
    _scrollController.dispose();
    _lineNumberScrollController.dispose();
    _controller.removeListener(_onTextChanged);
    _controller.dispose();
    _ragSearchController.dispose();
    super.dispose();
  }

  void _syncLineNumberScroll() {
    if (_lineNumberScrollController.hasClients) {
      _lineNumberScrollController.jumpTo(_scrollController.offset);
    }
  }

  void _onTextChanged() {
    _isDirty = true;
    widget.onChange?.call(_controller.text);
  }

  String get _fileName {
    if (widget.filePath == null) return '';
    final parts = widget.filePath!.split('/');
    return parts.last;
  }

  String get _fileDir {
    if (widget.filePath == null) return '';
    final parts = widget.filePath!.split('/');
    if (parts.length <= 1) return '';
    return parts.sublist(0, parts.length - 1).join('/');
  }

  String get _fileExtension {
    final name = _fileName;
    final idx = name.lastIndexOf('.');
    return idx >= 0 ? name.substring(idx) : '';
  }

  List<String> _buildLines() {
    if (_controller.text.isEmpty) return [''];
    return _controller.text.split('\n');
  }

  // ── RAG Search ──

  void _runRagSearch(String query) {
    setState(() {
      _ragSearchQuery = query;
      _ragMatches.clear();
      _activeMatchIndex = 0;
    });
    if (query.trim().isEmpty) return;

    final lines = _buildLines();
    final matches = <_RagSearchMatch>[];
    final q = query.toLowerCase();

    for (int i = 0; i < lines.length; i++) {
      final line = lines[i];
      final lineLower = line.toLowerCase();
      int col = 0;
      while (true) {
        final idx = lineLower.indexOf(q, col);
        if (idx < 0) break;
        matches.add(_RagSearchMatch(lineIdx: i, colStart: idx, colEnd: idx + q.length));
        col = idx + 1;
      }
    }

    // Compute relevance scores based on simple heuristics
    // (camelCase / snake_case boundary matches rank higher)
    for (final m in matches) {
      int score = 0;
      // Prefer matches near the start of identifiers
      if (m.colStart == 0) score += 5;
      // Prefer matches preceded by non-alphanumeric
      if (m.colStart > 0) {
        final prev = lines[m.lineIdx][m.colStart - 1];
        if (!RegExp(r'[a-zA-Z0-9]').hasMatch(prev)) score += 3;
      }
      m.score = score;
    }

    matches.sort((a, b) {
      // Higher score first, then line number
      final scoreCmp = b.score.compareTo(a.score);
      if (scoreCmp != 0) return scoreCmp;
      return a.lineIdx.compareTo(b.lineIdx);
    });

    setState(() {
      _ragMatches
        ..clear()
        ..addAll(matches);
    });
  }

  void _navigateRagMatch(int delta) {
    if (_ragMatches.isEmpty) return;
    setState(() {
      _activeMatchIndex = (_activeMatchIndex + delta) % _ragMatches.length;
      if (_activeMatchIndex < 0) _activeMatchIndex = _ragMatches.length - 1;
    });
    // Scroll to the match line
    final match = _ragMatches[_activeMatchIndex];
    final lineHeight = 20.0;
    if (_scrollController.hasClients) {
      _scrollController.animateTo(
        match.lineIdx * lineHeight - 100,
        duration: const Duration(milliseconds: 200),
        curve: Curves.easeInOut,
      );
    }
  }

  // ── Git Diff Gutter ──

  Widget _buildDiffGutter(int lineIndex) {
    // Placeholder: in a real implementation, query the provider for
    // GitDiffLine data. Here we randomly simulate a few diff markers.
    final content = _controller.text;
    if (content.isEmpty) return const SizedBox(width: 4);

    // Simulate some diff markers for demonstration
    // A real implementation would check provider.gitDiffs[filePath]
    final isModified = lineIndex > 0 && lineIndex % 7 == 0;
    final isAdded = lineIndex > 0 && lineIndex % 13 == 0 && !isModified;
    final isDeleted = lineIndex > 0 && lineIndex % 17 == 0 && !isModified;

    return Container(
      width: 4,
      margin: const EdgeInsets.only(right: 2),
      child: isModified
          ? Container(color: AppColors.darkWarning.withOpacity(0.7))
          : isAdded
              ? Container(color: AppColors.darkSuccess.withOpacity(0.7))
              : isDeleted
                  ? Container(
                      color: AppColors.darkDanger.withOpacity(0.7),
                      child: const Text('', style: TextStyle(fontSize: 0)),
                    )
                  : null,
    );
  }

  // ── Build ──

  @override
  Widget build(BuildContext context) {
    return Consumer<AppProvider>(
      builder: (context, provider, _) {
        final hasFile = widget.filePath != null;
        final isActiveProject = provider.activeProjectId.isNotEmpty;

        return Container(
          color: const Color(0xFF0D0D14),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // ── File Tabs Bar ──
              _buildFileTabsBar(provider, hasFile),

              // ── Editor Area ──
              Expanded(
                child: hasFile
                    ? _buildEditorContent(provider)
                    : _buildEmptyState(isActiveProject),
              ),

              // ── RAG Search Panel (collapsible) ──
              if (_showRagPanel) _buildRagSearchPanel(provider),
            ],
          ),
        );
      },
    );
  }

  Widget _buildFileTabsBar(AppProvider provider, bool hasFile) {
    final openFiles = provider.panes.isNotEmpty
        ? provider.panes[provider.activePaneIndex].openFiles
        : <String>[];

    return Container(
      height: 36,
      decoration: BoxDecoration(
        color: const Color(0xFF0D0D14),
        border: Border(
          bottom: BorderSide(color: AppColors.darkBorder.withOpacity(0.5)),
        ),
      ),
      child: Row(
        children: [
          // File tabs
          Expanded(
            child: ListView.builder(
              scrollDirection: Axis.horizontal,
              itemCount: openFiles.length,
              itemBuilder: (context, index) {
                final filePath = openFiles[index];
                final parts = filePath.split('/');
                final fileName = parts.last;
                final isActive = filePath == provider.activePaneFilePath;

                return DragTarget<int>(
                  onWillAcceptWithDetails: (details) {
                    setState(() => _dragOverPaneIdx = index);
                    return true;
                  },
                  onLeave: (value) => setState(() => _dragOverPaneIdx = null),
                  onAcceptWithDetails: (details) {
                    setState(() => _dragOverPaneIdx = null);
                    provider.handleDrop(details.data, provider.activePaneIndex, filePath);
                  },
                  builder: (context, candidateData, rejectedData) {
                    final isDragOver = _dragOverPaneIdx == index;

                    return Draggable<int>(
                      data: provider.activePaneIndex,
                      feedback: Material(
                        color: Colors.transparent,
                        child: Container(
                          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                          decoration: BoxDecoration(
                            color: AppColors.darkAccent.withOpacity(0.3),
                            borderRadius: BorderRadius.circular(4),
                            border: Border.all(color: AppColors.darkAccent),
                          ),
                          child: Text(
                            fileName,
                            style: const TextStyle(
                              color: AppColors.darkTextPrimary,
                              fontSize: 11,
                            ),
                          ),
                        ),
                      ),
                      childWhenDragging: Container(
                        height: 36,
                        width: 80,
                        decoration: BoxDecoration(
                          border: Border(
                            bottom: BorderSide(
                              color: AppColors.darkAccent.withOpacity(0.3),
                              width: 2,
                            ),
                          ),
                        ),
                      ),
                      child: GestureDetector(
                        onTap: () => provider.handleFileOpen(filePath),
                        child: Container(
                          padding: const EdgeInsets.symmetric(horizontal: 12),
                          constraints: const BoxConstraints(maxWidth: 180),
                          decoration: BoxDecoration(
                            color: isActive
                                ? const Color(0xFF16161E)
                                : (isDragOver
                                    ? AppColors.darkAccent.withOpacity(0.15)
                                    : Colors.transparent),
                            border: Border(
                              bottom: BorderSide(
                                color: isActive
                                    ? AppColors.darkAccent
                                    : Colors.transparent,
                                width: 2,
                              ),
                            ),
                          ),
                          child: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              Icon(
                                Icons.description_outlined,
                                size: 13,
                                color: isActive
                                    ? AppColors.darkAccent
                                    : AppColors.darkTextSecondary,
                              ),
                              const SizedBox(width: 6),
                              Flexible(
                                child: Text(
                                  fileName,
                                  overflow: TextOverflow.ellipsis,
                                  style: TextStyle(
                                    color: isActive
                                        ? AppColors.darkTextPrimary
                                        : AppColors.darkTextSecondary,
                                    fontSize: 12,
                                  ),
                                ),
                              ),
                              // Dirty indicator dot
                              if (_isDirty && isActive)
                                Container(
                                  width: 6,
                                  height: 6,
                                  margin: const EdgeInsets.only(left: 4),
                                  decoration: const BoxDecoration(
                                    shape: BoxShape.circle,
                                    color: AppColors.darkWarning,
                                  ),
                                ),
                              if (openFiles.length > 1) ...[
                                const SizedBox(width: 6),
                                GestureDetector(
                                  onTap: () => provider.handleFileClose(
                                    provider.activePaneIndex,
                                    filePath,
                                  ),
                                  child: MouseRegion(
                                    cursor: SystemMouseCursors.click,
                                    child: Icon(
                                      Icons.close,
                                      size: 13,
                                      color: isActive
                                          ? AppColors.darkTextSecondary
                                          : Colors.transparent,
                                    ),
                                  ),
                                ),
                              ],
                            ],
                          ),
                        ),
                      ),
                    );
                  },
                );
              },
            ),
          ),

          // Split button
          GestureDetector(
            onTap: provider.handleSplit,
            child: MouseRegion(
              cursor: SystemMouseCursors.click,
              child: Container(
                padding: const EdgeInsets.all(4),
                margin: const EdgeInsets.symmetric(horizontal: 2),
                child: Icon(
                  Icons.vertical_split_rounded,
                  size: 14,
                  color: AppColors.darkTextSecondary.withOpacity(0.6),
                ),
              ),
            ),
          ),

          // RAG search toggle
          GestureDetector(
            onTap: () => setState(() => _showRagPanel = !_showRagPanel),
            child: MouseRegion(
              cursor: SystemMouseCursors.click,
              child: Container(
                padding: const EdgeInsets.all(4),
                margin: const EdgeInsets.symmetric(horizontal: 2),
                decoration: BoxDecoration(
                  color: _showRagPanel
                      ? AppColors.darkAccent.withOpacity(0.15)
                      : null,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Icon(
                  Icons.search_rounded,
                  size: 14,
                  color: _showRagPanel
                      ? AppColors.darkAccent
                      : AppColors.darkTextSecondary.withOpacity(0.6),
                ),
              ),
            ),
          ),

          // Save button
          if (hasFile)
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 6),
              child: GestureDetector(
                onTap: _handleSave,
                child: MouseRegion(
                  cursor: SystemMouseCursors.click,
                  child: Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 8,
                      vertical: 4,
                    ),
                    decoration: BoxDecoration(
                      color: _isDirty
                          ? AppColors.darkAccent.withOpacity(0.15)
                          : AppColors.darkSuccess.withOpacity(0.1),
                      borderRadius: BorderRadius.circular(4),
                      border: Border.all(
                        color: _isDirty
                            ? AppColors.darkAccent.withOpacity(0.3)
                            : AppColors.darkSuccess.withOpacity(0.2),
                      ),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(
                          _isDirty ? Icons.circle_outlined : Icons.save_outlined,
                          size: 12,
                          color: _isDirty
                              ? AppColors.darkAccent
                              : AppColors.darkSuccess,
                        ),
                        const SizedBox(width: 4),
                        Text(
                          _isDirty ? 'Unsaved' : 'Saved',
                          style: TextStyle(
                            color: _isDirty
                                ? AppColors.darkAccent
                                : AppColors.darkSuccess,
                            fontSize: 10,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }

  Widget _buildEditorContent(AppProvider provider) {
    if (_isLoading) return _buildLoadingState();
    if (_errorMessage != null) return _buildErrorState();

    final lines = _buildLines();
    final lineCount = lines.length;

    return Row(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // ── Line Numbers & Git Diff Gutter ──
        Container(
          width: 56,
          color: const Color(0xFF0D0D14),
          child: ListView.builder(
            controller: _lineNumberScrollController,
            physics: const NeverScrollableScrollPhysics(),
            itemCount: lineCount,
            itemExtent: 20,
            itemBuilder: (context, index) {
              return Row(
                children: [
                  // Git diff gutter indicator
                  _buildDiffGutter(index),

                  // Line number
                  Expanded(
                    child: Container(
                      alignment: Alignment.centerRight,
                      padding: const EdgeInsets.only(right: 12),
                      decoration: BoxDecoration(
                        border: Border(
                          right: BorderSide(
                            color: AppColors.darkBorder.withOpacity(0.4),
                          ),
                        ),
                      ),
                      child: Text(
                        '${index + 1}',
                        style: const TextStyle(
                          color: Color(0xFF555570),
                          fontSize: 11,
                          fontFamily: 'monospace',
                          height: 1.5,
                        ),
                      ),
                    ),
                  ),
                ],
              );
            },
          ),
        ),

        // ── File Header Bar & Editor ──
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // File info header
              _buildFileHeader(provider),

              // Text editor
              Expanded(
                child: TextField(
                  controller: _controller,
                  scrollController: _scrollController,
                  maxLines: null,
                  expands: true,
                  textAlignVertical: TextAlignVertical.top,
                  style: const TextStyle(
                    color: Color(0xFFD4D4DC),
                    fontSize: 13,
                    fontFamily: 'monospace',
                    height: 1.5,
                  ),
                  decoration: const InputDecoration(
                    border: InputBorder.none,
                    contentPadding: EdgeInsets.only(
                      left: 12,
                      right: 12,
                      top: 4,
                      bottom: 4,
                    ),
                    isDense: true,
                  ),
                  keyboardType: TextInputType.multiline,
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildFileHeader(AppProvider provider) {
    final lines = _buildLines();
    final lineCount = lines.length;
    final charCount = _controller.text.length;

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
      decoration: BoxDecoration(
        color: const Color(0xFF12121A),
        border: Border(
          bottom: BorderSide(
            color: AppColors.darkBorder.withOpacity(0.3),
          ),
        ),
      ),
      child: Row(
        children: [
          Icon(
            Icons.insert_drive_file_outlined,
            size: 14,
            color: AppColors.darkAccent,
          ),
          const SizedBox(width: 6),
          Flexible(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  _fileName,
                  style: const TextStyle(
                    color: Color(0xFFD4D4DC),
                    fontSize: 12,
                    fontWeight: FontWeight.w600,
                    fontFamily: 'monospace',
                  ),
                ),
                if (_fileDir.isNotEmpty)
                  Text(
                    _fileDir,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(
                      color: AppColors.darkTextSecondary.withOpacity(0.7),
                      fontSize: 10,
                      fontFamily: 'monospace',
                    ),
                  ),
              ],
            ),
          ),
          const Spacer(),
          // Cursor position placeholder
          Text(
            'Ln 1, Col 1',
            style: TextStyle(
              color: AppColors.darkTextSecondary.withOpacity(0.5),
              fontSize: 10,
              fontFamily: 'monospace',
            ),
          ),
          const SizedBox(width: 12),
          // File stats
          Text(
            '$lineCount lines · ${_formatBytes(charCount)}',
            style: TextStyle(
              color: AppColors.darkTextSecondary.withOpacity(0.5),
              fontSize: 10,
              fontFamily: 'monospace',
            ),
          ),
          const SizedBox(width: 12),
          // Language / extension badge
          if (_fileExtension.isNotEmpty)
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
              decoration: BoxDecoration(
                color: AppColors.darkAccent.withOpacity(0.1),
                borderRadius: BorderRadius.circular(3),
              ),
              child: Text(
                _fileExtension,
                style: TextStyle(
                  color: AppColors.darkAccent.withOpacity(0.7),
                  fontSize: 9,
                  fontFamily: 'monospace',
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
          const SizedBox(width: 12),
          Text(
            _formatTimestamp(DateTime.now()),
            style: TextStyle(
              color: AppColors.darkTextSecondary.withOpacity(0.5),
              fontSize: 10,
              fontFamily: 'monospace',
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildEmptyState(bool isActiveProject) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.code_off_outlined,
            size: 48,
            color: AppColors.darkTextSecondary.withOpacity(0.3),
          ),
          const SizedBox(height: 16),
          Text(
            'No File Selected',
            style: TextStyle(
              color: AppColors.darkTextSecondary.withOpacity(0.6),
              fontSize: 15,
              fontWeight: FontWeight.w500,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            isActiveProject
                ? 'Select a file from the explorer to start editing'
                : 'Open a project to browse and edit files',
            style: TextStyle(
              color: AppColors.darkTextSecondary.withOpacity(0.4),
              fontSize: 12,
            ),
          ),
          const SizedBox(height: 24),
          // Split pane hint
          GestureDetector(
            onTap: () => context.read<AppProvider>().handleSplit(),
            child: MouseRegion(
              cursor: SystemMouseCursors.click,
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                decoration: BoxDecoration(
                  color: AppColors.darkAccent.withOpacity(0.1),
                  borderRadius: BorderRadius.circular(6),
                  border: Border.all(
                    color: AppColors.darkAccent.withOpacity(0.2),
                  ),
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      Icons.vertical_split_rounded,
                      size: 14,
                      color: AppColors.darkAccent,
                    ),
                    const SizedBox(width: 6),
                    Text(
                      'Split Editor',
                      style: TextStyle(
                        color: AppColors.darkAccent,
                        fontSize: 11,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildLoadingState() {
    return const Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          SizedBox(
            width: 28,
            height: 28,
            child: CircularProgressIndicator(
              strokeWidth: 2,
              color: AppColors.darkAccent,
            ),
          ),
          SizedBox(height: 12),
          Text(
            'Loading file...',
            style: TextStyle(
              color: AppColors.darkTextSecondary,
              fontSize: 12,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildErrorState() {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.error_outline_rounded,
            size: 40,
            color: AppColors.darkDanger.withOpacity(0.5),
          ),
          const SizedBox(height: 12),
          Text(
            'Failed to load file',
            style: TextStyle(
              color: AppColors.darkDanger.withOpacity(0.7),
              fontSize: 14,
              fontWeight: FontWeight.w500,
            ),
          ),
          if (_errorMessage != null) ...[
            const SizedBox(height: 4),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 32),
              child: Text(
                _errorMessage!,
                textAlign: TextAlign.center,
                style: TextStyle(
                  color: AppColors.darkTextSecondary.withOpacity(0.6),
                  fontSize: 11,
                ),
              ),
            ),
          ],
          const SizedBox(height: 16),
          GestureDetector(
            onTap: () {
              setState(() {
                _errorMessage = null;
                _isLoading = false;
              });
            },
            child: MouseRegion(
              cursor: SystemMouseCursors.click,
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                decoration: BoxDecoration(
                  color: AppColors.darkDanger.withOpacity(0.1),
                  borderRadius: BorderRadius.circular(6),
                  border: Border.all(
                    color: AppColors.darkDanger.withOpacity(0.2),
                  ),
                ),
                child: Text(
                  'Retry',
                  style: TextStyle(
                    color: AppColors.darkDanger.withOpacity(0.7),
                    fontSize: 11,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  // ── RAG Search Panel ──

  Widget _buildRagSearchPanel(AppProvider provider) {
    return Container(
      height: 44,
      decoration: BoxDecoration(
        color: const Color(0xFF12121A),
        border: Border(
          top: BorderSide(color: AppColors.darkBorder.withOpacity(0.4)),
        ),
      ),
      child: Row(
        children: [
          const SizedBox(width: 8),
          // Search icon
          Icon(
            Icons.search_rounded,
            size: 14,
            color: AppColors.darkAccent,
          ),
          const SizedBox(width: 6),
          // Input field
          Expanded(
            child: TextField(
              controller: _ragSearchController,
              autofocus: true,
              onChanged: _runRagSearch,
              style: const TextStyle(
                color: Color(0xFFD4D4DC),
                fontSize: 11,
                fontFamily: 'monospace',
              ),
              decoration: InputDecoration(
                hintText: 'Code search (RAG)...',
                hintStyle: TextStyle(
                  color: AppColors.darkTextSecondary.withOpacity(0.4),
                  fontSize: 11,
                ),
                border: InputBorder.none,
                isDense: true,
                contentPadding: const EdgeInsets.symmetric(vertical: 8),
              ),
              onSubmitted: (v) {
                if (_ragMatches.isNotEmpty) {
                  final match = _ragMatches.first;
                  provider.handleFileOpen(widget.filePath ?? '', line: match.lineIdx + 1);
                }
              },
            ),
          ),
          // Match count
          if (_ragSearchQuery.isNotEmpty)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 6),
              child: Text(
                _ragMatches.isEmpty
                    ? 'No results'
                    : '${_activeMatchIndex + 1}/${_ragMatches.length}',
                style: TextStyle(
                  color: _ragMatches.isEmpty
                      ? AppColors.darkDanger.withOpacity(0.6)
                      : AppColors.darkAccent,
                  fontSize: 10,
                  fontFamily: 'monospace',
                ),
              ),
            ),
          // Navigation buttons
          if (_ragMatches.length > 1) ...[
            _RagNavButton(
              icon: Icons.keyboard_arrow_up_rounded,
              onTap: () => _navigateRagMatch(-1),
            ),
            _RagNavButton(
              icon: Icons.keyboard_arrow_down_rounded,
              onTap: () => _navigateRagMatch(1),
            ),
          ],
          // Close
          GestureDetector(
            onTap: () {
              setState(() {
                _showRagPanel = false;
                _ragSearchController.clear();
                _ragSearchQuery = '';
                _ragMatches.clear();
              });
            },
            child: MouseRegion(
              cursor: SystemMouseCursors.click,
              child: Container(
                padding: const EdgeInsets.all(6),
                child: Icon(
                  Icons.close_rounded,
                  size: 14,
                  color: AppColors.darkTextSecondary.withOpacity(0.5),
                ),
              ),
            ),
          ),
          const SizedBox(width: 4),
        ],
      ),
    );
  }

  // ── Save ──

  void _handleSave() {
    if (widget.filePath == null) return;
    final provider = context.read<AppProvider>();
    provider.setFileContent(widget.filePath!, _controller.text);
    setState(() => _isDirty = false);

    provider.showToast(
      'File saved: $_fileName',
      type: 'success',
    );
  }

  // ── Helpers ──

  String _formatTimestamp(DateTime dt) {
    return '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}:${dt.second.toString().padLeft(2, '0')}';
  }

  String _formatBytes(int bytes) {
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
  }
}

/// Small button for RAG search result navigation.
class _RagNavButton extends StatelessWidget {
  final IconData icon;
  final VoidCallback onTap;

  const _RagNavButton({required this.icon, required this.onTap});

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onTap,
      child: MouseRegion(
        cursor: SystemMouseCursors.click,
        child: Container(
          padding: const EdgeInsets.all(4),
          margin: const EdgeInsets.symmetric(horizontal: 1),
          decoration: BoxDecoration(
            color: AppColors.darkAccent.withOpacity(0.1),
            borderRadius: BorderRadius.circular(3),
          ),
          child: Icon(
            icon,
            size: 14,
            color: AppColors.darkAccent,
          ),
        ),
      ),
    );
  }
}

/// A match result from the code RAG search.
class _RagSearchMatch {
  final int lineIdx;
  final int colStart;
  final int colEnd;
  int score = 0;

  _RagSearchMatch({
    required this.lineIdx,
    required this.colStart,
    required this.colEnd,
  });
}
