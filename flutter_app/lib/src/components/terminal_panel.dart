import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/app_provider.dart';
import '../theme/app_theme.dart';
import '../models/project.dart';

/// Terminal panel showing process output logs with session management,
/// filter/search, and an interactive terminal input bar.
class TerminalPanelWidget extends StatefulWidget {
  const TerminalPanelWidget({super.key});

  @override
  State<TerminalPanelWidget> createState() => _TerminalPanelWidgetState();
}

class _TerminalPanelWidgetState extends State<TerminalPanelWidget> {
  final TextEditingController _inputController = TextEditingController();
  final TextEditingController _searchController = TextEditingController();
  final ScrollController _logScrollController = ScrollController();
  String _activeFilter = 'all'; // all, stdout, stderr, system
  bool _autoScroll = true;
  bool _showSearch = false;
  String _logSearchQuery = '';

  /// View mode: 'log' shows the log output, 'terminal' shows a terminal
  /// placeholder (simulating an xterm-like area).
  String _viewMode = 'log';

  @override
  void initState() {
    super.initState();
    _logScrollController.addListener(_onScrollChanged);
  }

  @override
  void dispose() {
    _inputController.dispose();
    _searchController.dispose();
    _logScrollController.removeListener(_onScrollChanged);
    _logScrollController.dispose();
    super.dispose();
  }

  void _onScrollChanged() {
    if (_logScrollController.hasClients) {
      final isAtBottom = _logScrollController.position.pixels >=
          _logScrollController.position.maxScrollExtent - 50;
      if (_autoScroll != isAtBottom) {
        setState(() => _autoScroll = isAtBottom);
      }
    }
  }

  void _scrollToBottom() {
    if (_autoScroll && _logScrollController.hasClients) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (_logScrollController.hasClients) {
          _logScrollController.jumpTo(
            _logScrollController.position.maxScrollExtent,
          );
        }
      });
    }
  }

  Color _streamColor(String stream) {
    switch (stream) {
      case 'stderr':
        return AppColors.darkDanger;
      case 'system':
        return AppColors.darkInfo;
      default:
        return AppColors.darkTextPrimary;
    }
  }

  String _streamLabel(String stream) {
    switch (stream) {
      case 'stdout':
        return 'OUT';
      case 'stderr':
        return 'ERR';
      case 'system':
        return 'SYS';
      default:
        return stream.toUpperCase();
    }
  }

  @override
  Widget build(BuildContext context) {
    return Consumer<AppProvider>(
      builder: (context, provider, _) {
        final logs = provider.activeProjectId.isNotEmpty
            ? (provider.projectLogs[provider.activeProjectId] ?? [])
            : <LogLine>[];
        final filteredLogs = _activeFilter == 'all'
            ? logs
            : logs.where((l) => l.stream == _activeFilter).toList();

        // Apply local log text search
        final searchedLogs = _logSearchQuery.isEmpty
            ? filteredLogs
            : filteredLogs
                .where((l) =>
                    l.text.toLowerCase().contains(_logSearchQuery.toLowerCase()))
                .toList();

        final hasTerminals = provider.terminals.isNotEmpty;

        return Container(
          color: const Color(0xFF0A0A10),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // ── Toolbar ──
              _buildToolbar(provider, logs.length, searchedLogs.length),

              // ── Main Content ──
              Expanded(
                child: _viewMode == 'log'
                    ? (hasTerminals
                        ? Row(
                            crossAxisAlignment: CrossAxisAlignment.stretch,
                            children: [
                              // Terminal session sidebar
                              _buildSessionSidebar(provider),

                              // Log area
                              Expanded(
                                child: _buildLogArea(searchedLogs),
                              ),
                            ],
                          )
                        : _buildEmptyState())
                    : _buildTerminalArea(provider),
              ),

              // ── Search bar (collapsible) ──
              if (_showSearch) _buildSearchBar(),

              // ── Input Bar ──
              _buildInputBar(provider, hasTerminals),
            ],
          ),
        );
      },
    );
  }

  Widget _buildToolbar(AppProvider provider, int totalLogs, int searchedCount) {
    final filterCounts = {
      'all': totalLogs,
      'stdout':
          provider.projectLogs[provider.activeProjectId]
                  ?.where((l) => l.stream == 'stdout')
                  .length ??
              0,
      'stderr':
          provider.projectLogs[provider.activeProjectId]
                  ?.where((l) => l.stream == 'stderr')
                  .length ??
              0,
      'system':
          provider.projectLogs[provider.activeProjectId]
                  ?.where((l) => l.stream == 'system')
                  .length ??
              0,
    };

    return Container(
      height: 40,
      padding: const EdgeInsets.symmetric(horizontal: 8),
      decoration: BoxDecoration(
        color: const Color(0xFF0E0E16),
        border: Border(
          bottom: BorderSide(color: AppColors.darkBorder.withOpacity(0.5)),
        ),
      ),
      child: Row(
        children: [
          // View mode toggle
          _ViewModePill(
            label: 'Log',
            icon: Icons.terminal_rounded,
            isActive: _viewMode == 'log',
            onTap: () => setState(() => _viewMode = 'log'),
          ),
          const SizedBox(width: 4),
          _ViewModePill(
            label: 'Terminal',
            icon: Icons.dns_outlined,
            isActive: _viewMode == 'terminal',
            onTap: () => setState(() => _viewMode = 'terminal'),
          ),

          const SizedBox(width: 8),

          // Terminal tab selector (only in log mode)
          if (_viewMode == 'log' && provider.terminals.isNotEmpty)
            Expanded(
              child: ListView.builder(
                scrollDirection: Axis.horizontal,
                itemCount: provider.terminals.length,
                itemBuilder: (context, index) {
                  final term = provider.terminals[index];
                  final isActive = term.id == provider.activeTerminalId;

                  return GestureDetector(
                    onTap: () => provider.setActiveTerminalId(term.id),
                    child: Container(
                      margin: const EdgeInsets.symmetric(horizontal: 2),
                      padding: const EdgeInsets.symmetric(horizontal: 10),
                      decoration: BoxDecoration(
                        color: isActive
                            ? const Color(0xFF1A1A28)
                            : Colors.transparent,
                        borderRadius: BorderRadius.circular(4),
                        border: Border.all(
                          color: isActive
                              ? AppColors.darkAccent.withOpacity(0.4)
                              : AppColors.darkBorder.withOpacity(0.3),
                        ),
                      ),
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Icon(
                            Icons.terminal,
                            size: 12,
                            color: isActive
                                ? AppColors.darkAccent
                                : AppColors.darkTextSecondary,
                          ),
                          const SizedBox(width: 4),
                          Text(
                            term.name,
                            style: TextStyle(
                              color: isActive
                                  ? AppColors.darkTextPrimary
                                  : AppColors.darkTextSecondary,
                              fontSize: 11,
                            ),
                          ),
                          const SizedBox(width: 4),
                          GestureDetector(
                            onTap: () =>
                                _handleKillTerminal(provider, term.id),
                            child: Icon(
                              Icons.close,
                              size: 11,
                              color: AppColors.darkTextSecondary
                                  .withOpacity(0.5),
                            ),
                          ),
                        ],
                      ),
                    ),
                  );
                },
              ),
            ),

          if (_viewMode == 'log') ...[
            // Filter pills
            ...['all', 'stdout', 'stderr', 'system'].map((filter) {
              final isActive = _activeFilter == filter;
              final count = filterCounts[filter] ?? 0;

              return GestureDetector(
                onTap: () => setState(() => _activeFilter = filter),
                child: Container(
                  margin: const EdgeInsets.symmetric(horizontal: 2),
                  padding:
                      const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                  decoration: BoxDecoration(
                    color: isActive
                        ? _filterColor(filter).withOpacity(0.15)
                        : Colors.transparent,
                    borderRadius: BorderRadius.circular(4),
                    border: Border.all(
                      color: isActive
                          ? _filterColor(filter).withOpacity(0.4)
                          : AppColors.darkBorder.withOpacity(0.2),
                    ),
                  ),
                  child: Text(
                    '${filter == 'all' ? 'ALL' : filter.toUpperCase()} ($count)',
                    style: TextStyle(
                      color: isActive
                          ? _filterColor(filter)
                          : AppColors.darkTextSecondary,
                      fontSize: 10,
                      fontWeight:
                          isActive ? FontWeight.w600 : FontWeight.normal,
                    ),
                  ),
                ),
              );
            }),
          ],

          const SizedBox(width: 4),

          // Search toggle button
          GestureDetector(
            onTap: () => setState(() => _showSearch = !_showSearch),
            child: MouseRegion(
              cursor: SystemMouseCursors.click,
              child: Container(
                padding: const EdgeInsets.all(4),
                decoration: BoxDecoration(
                  color: _showSearch
                      ? AppColors.darkAccent.withOpacity(0.15)
                      : null,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Icon(
                  Icons.search_rounded,
                  size: 14,
                  color: _showSearch
                      ? AppColors.darkAccent
                      : AppColors.darkTextSecondary.withOpacity(0.6),
                ),
              ),
            ),
          ),

          const SizedBox(width: 2),

          // Clear button
          GestureDetector(
            onTap: () => _handleClear(provider),
            child: MouseRegion(
              cursor: SystemMouseCursors.click,
              child: Container(
                padding: const EdgeInsets.all(4),
                decoration: BoxDecoration(
                  color: AppColors.darkDanger.withOpacity(0.1),
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Icon(
                  Icons.delete_outline,
                  size: 14,
                  color: AppColors.darkDanger.withOpacity(0.7),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Color _filterColor(String filter) {
    switch (filter) {
      case 'stdout':
        return AppColors.darkTextPrimary;
      case 'stderr':
        return AppColors.darkDanger;
      case 'system':
        return AppColors.darkInfo;
      default:
        return AppColors.darkAccent;
    }
  }

  Widget _buildSearchBar() {
    return Container(
      height: 32,
      padding: const EdgeInsets.symmetric(horizontal: 8),
      decoration: BoxDecoration(
        color: const Color(0xFF0C0C14),
        border: Border(
          bottom: BorderSide(color: AppColors.darkBorder.withOpacity(0.3)),
        ),
      ),
      child: Row(
        children: [
          Icon(
            Icons.search_rounded,
            size: 13,
            color: AppColors.darkAccent,
          ),
          const SizedBox(width: 6),
          Expanded(
            child: TextField(
              controller: _searchController,
              onChanged: (v) => setState(() => _logSearchQuery = v),
              style: const TextStyle(
                color: Color(0xFFD4D4DC),
                fontSize: 11,
                fontFamily: 'monospace',
              ),
              decoration: InputDecoration(
                hintText: 'Filter log output...',
                hintStyle: TextStyle(
                  color: AppColors.darkTextSecondary.withOpacity(0.4),
                  fontSize: 11,
                ),
                border: InputBorder.none,
                isDense: true,
                contentPadding: const EdgeInsets.symmetric(vertical: 6),
              ),
            ),
          ),
          if (_logSearchQuery.isNotEmpty)
            GestureDetector(
              onTap: () {
                _searchController.clear();
                setState(() => _logSearchQuery = '');
              },
              child: MouseRegion(
                cursor: SystemMouseCursors.click,
                child: Padding(
                  padding: const EdgeInsets.all(4),
                  child: Icon(
                    Icons.close_rounded,
                    size: 13,
                    color: AppColors.darkTextSecondary.withOpacity(0.5),
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }

  Widget _buildSessionSidebar(AppProvider provider) {
    return Container(
      width: 160,
      decoration: BoxDecoration(
        color: const Color(0xFF0C0C14),
        border: Border(
          right: BorderSide(
            color: AppColors.darkBorder.withOpacity(0.3),
          ),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Header
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
            decoration: BoxDecoration(
              border: Border(
                bottom: BorderSide(
                  color: AppColors.darkBorder.withOpacity(0.2),
                ),
              ),
            ),
            child: Text(
              'TERMINALS',
              style: TextStyle(
                color: AppColors.darkTextSecondary.withOpacity(0.6),
                fontSize: 10,
                fontWeight: FontWeight.w700,
                letterSpacing: 1,
              ),
            ),
          ),

          // Session list
          Expanded(
            child: ListView.builder(
              itemCount: provider.terminals.length,
              itemBuilder: (context, index) {
                final term = provider.terminals[index];
                final isActive = term.id == provider.activeTerminalId;

                return GestureDetector(
                  onTap: () => provider.setActiveTerminalId(term.id),
                  child: Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 10,
                      vertical: 8,
                    ),
                    decoration: BoxDecoration(
                      color: isActive
                          ? AppColors.darkAccent.withOpacity(0.1)
                          : Colors.transparent,
                      border: Border(
                        left: BorderSide(
                          color: isActive
                              ? AppColors.darkAccent
                              : Colors.transparent,
                          width: 2,
                        ),
                      ),
                    ),
                    child: Row(
                      children: [
                        Container(
                          width: 6,
                          height: 6,
                          decoration: BoxDecoration(
                            shape: BoxShape.circle,
                            color: isActive
                                ? AppColors.darkSuccess
                                : AppColors.darkTextSecondary.withOpacity(0.3),
                          ),
                        ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            term.name,
                            overflow: TextOverflow.ellipsis,
                            style: TextStyle(
                              color: isActive
                                  ? AppColors.darkTextPrimary
                                  : AppColors.darkTextSecondary,
                              fontSize: 11,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                );
              },
            ),
          ),

          // Spawn new terminal button
          Container(
            padding: const EdgeInsets.all(6),
            child: GestureDetector(
              onTap: () => _handleSpawnTerminal(provider),
              child: MouseRegion(
                cursor: SystemMouseCursors.click,
                child: Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 8,
                    vertical: 6,
                  ),
                  decoration: BoxDecoration(
                    color: AppColors.darkAccent.withOpacity(0.1),
                    borderRadius: BorderRadius.circular(4),
                    border: Border.all(
                      color: AppColors.darkAccent.withOpacity(0.2),
                    ),
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(
                        Icons.add,
                        size: 12,
                        color: AppColors.darkAccent,
                      ),
                      const SizedBox(width: 4),
                      Text(
                        'New Terminal',
                        style: TextStyle(
                          color: AppColors.darkAccent,
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

  Widget _buildLogArea(List<LogLine> logs) {
    if (logs.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.terminal,
              size: 32,
              color: AppColors.darkTextSecondary.withOpacity(0.2),
            ),
            const SizedBox(height: 8),
            Text(
              _logSearchQuery.isNotEmpty
                  ? 'No output matches your search'
                  : 'No output yet',
              style: TextStyle(
                color: AppColors.darkTextSecondary.withOpacity(0.4),
                fontSize: 12,
              ),
            ),
          ],
        ),
      );
    }

    return ListView.builder(
      controller: _logScrollController,
      padding: const EdgeInsets.symmetric(vertical: 4),
      itemCount: logs.length,
      itemExtent: 22,
      itemBuilder: (context, index) {
        final log = logs[index];
        final time = DateTime.fromMillisecondsSinceEpoch(log.timestamp);
        final timeStr = '${time.hour.toString().padLeft(2, '0')}:${time.minute.toString().padLeft(2, '0')}:${time.second.toString().padLeft(2, '0')}';

        return Container(
          padding: const EdgeInsets.symmetric(horizontal: 8),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              // Timestamp
              SizedBox(
                width: 64,
                child: Text(
                  timeStr,
                  style: TextStyle(
                    color: AppColors.darkTextSecondary.withOpacity(0.4),
                    fontSize: 10,
                    fontFamily: 'monospace',
                  ),
                ),
              ),

              // Stream badge
              Container(
                width: 30,
                margin: const EdgeInsets.only(right: 6),
                padding:
                    const EdgeInsets.symmetric(horizontal: 3, vertical: 1),
                decoration: BoxDecoration(
                  color: _streamColor(log.stream).withOpacity(0.15),
                  borderRadius: BorderRadius.circular(2),
                ),
                child: Text(
                  _streamLabel(log.stream),
                  style: TextStyle(
                    color: _streamColor(log.stream).withOpacity(0.8),
                    fontSize: 8,
                    fontWeight: FontWeight.w700,
                    fontFamily: 'monospace',
                  ),
                ),
              ),

              // Log text
              Expanded(
                child: Text(
                  log.text,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    color: _streamColor(log.stream),
                    fontSize: 12,
                    fontFamily: 'monospace',
                    height: 1.4,
                  ),
                ),
              ),
            ],
          ),
        );
      },
    );
  }

  Widget _buildTerminalArea(AppProvider provider) {
    final hasTerminals = provider.terminals.isNotEmpty;

    if (!hasTerminals) {
      return _buildEmptyState();
    }

    // Placeholder interactive terminal area (xterm-like).
    // In a real implementation, this would be a webview or custom
    // terminal emulator connected via FRB.
    return Container(
      color: const Color(0xFF0A0A10),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Terminal session tabs
          Container(
            height: 28,
            padding: const EdgeInsets.symmetric(horizontal: 8),
            decoration: BoxDecoration(
              color: const Color(0xFF0E0E16),
              border: Border(
                bottom: BorderSide(
                  color: AppColors.darkBorder.withOpacity(0.3),
                ),
              ),
            ),
            child: Row(
              children: provider.terminals.map((term) {
                final isActive = term.id == provider.activeTerminalId;
                return GestureDetector(
                  onTap: () => provider.setActiveTerminalId(term.id),
                  child: Container(
                    margin: const EdgeInsets.only(right: 4),
                    padding: const EdgeInsets.symmetric(
                      horizontal: 10,
                      vertical: 4,
                    ),
                    decoration: BoxDecoration(
                      color: isActive
                          ? const Color(0xFF1A1A28)
                          : Colors.transparent,
                      borderRadius: BorderRadius.circular(4),
                      border: Border.all(
                        color: isActive
                            ? AppColors.darkAccent.withOpacity(0.4)
                            : AppColors.darkBorder.withOpacity(0.2),
                      ),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(
                          Icons.dns_outlined,
                          size: 11,
                          color: isActive
                              ? AppColors.darkAccent
                              : AppColors.darkTextSecondary,
                        ),
                        const SizedBox(width: 4),
                        Text(
                          term.name,
                          style: TextStyle(
                            color: isActive
                                ? AppColors.darkTextPrimary
                                : AppColors.darkTextSecondary,
                            fontSize: 10,
                          ),
                        ),
                      ],
                    ),
                  ),
                );
              }).toList(),
            ),
          ),

          // Terminal emulator placeholder
          Expanded(
            child: Container(
              padding: const EdgeInsets.all(12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  // Terminal header line
                  Row(
                    children: [
                      Container(
                        width: 8,
                        height: 8,
                        decoration: const BoxDecoration(
                          shape: BoxShape.circle,
                          color: AppColors.darkSuccess,
                        ),
                      ),
                      const SizedBox(width: 6),
                      Text(
                        'Terminal active — interactive mode',
                        style: TextStyle(
                          color: AppColors.darkSuccess.withOpacity(0.7),
                          fontSize: 10,
                          fontFamily: 'monospace',
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 16),

                  // Simulated terminal lines
                  Text(
                    r'$ ',
                    style: const TextStyle(
                      color: Color(0xFFD4D4DC),
                      fontSize: 13,
                      fontFamily: 'monospace',
                      height: 1.6,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    '  █ Interactive terminal',
                    style: TextStyle(
                      color: AppColors.darkTextSecondary.withOpacity(0.3),
                      fontSize: 11,
                      fontFamily: 'monospace',
                    ),
                  ),
                  const Spacer(),
                  // Info bar
                  Container(
                    padding: const EdgeInsets.all(8),
                    decoration: BoxDecoration(
                      color: AppColors.darkAccent.withOpacity(0.08),
                      borderRadius: BorderRadius.circular(6),
                      border: Border.all(
                        color: AppColors.darkAccent.withOpacity(0.15),
                      ),
                    ),
                    child: Row(
                      children: [
                        Icon(
                          Icons.info_outline,
                          size: 12,
                          color: AppColors.darkAccent.withOpacity(0.6),
                        ),
                        const SizedBox(width: 6),
                        Text(
                          'FRB webview terminal coming soon',
                          style: TextStyle(
                            color: AppColors.darkAccent.withOpacity(0.5),
                            fontSize: 10,
                            fontFamily: 'monospace',
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildEmptyState() {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.terminal_outlined,
            size: 48,
            color: AppColors.darkTextSecondary.withOpacity(0.3),
          ),
          const SizedBox(height: 16),
          Text(
            'No Active Terminals',
            style: TextStyle(
              color: AppColors.darkTextSecondary.withOpacity(0.6),
              fontSize: 15,
              fontWeight: FontWeight.w500,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            'Spawn a terminal session to view or send commands',
            style: TextStyle(
              color: AppColors.darkTextSecondary.withOpacity(0.4),
              fontSize: 12,
            ),
          ),
          const SizedBox(height: 20),
          GestureDetector(
            onTap: () => _handleSpawnTerminal(context.read<AppProvider>()),
            child: MouseRegion(
              cursor: SystemMouseCursors.click,
              child: Container(
                padding: const EdgeInsets.symmetric(
                  horizontal: 16,
                  vertical: 8,
                ),
                decoration: BoxDecoration(
                  color: AppColors.darkAccent.withOpacity(0.15),
                  borderRadius: BorderRadius.circular(6),
                  border: Border.all(
                    color: AppColors.darkAccent.withOpacity(0.3),
                  ),
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      Icons.add_circle_outline,
                      size: 16,
                      color: AppColors.darkAccent,
                    ),
                    const SizedBox(width: 6),
                    Text(
                      'Create Terminal',
                      style: TextStyle(
                        color: AppColors.darkAccent,
                        fontSize: 13,
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

  Widget _buildInputBar(AppProvider provider, bool hasTerminals) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
      decoration: BoxDecoration(
        color: const Color(0xFF0E0E16),
        border: Border(
          top: BorderSide(
            color: AppColors.darkBorder.withOpacity(0.5),
          ),
        ),
      ),
      child: Row(
        children: [
          Icon(
            Icons.chevron_right,
            size: 14,
            color: hasTerminals
                ? AppColors.darkSuccess
                : AppColors.darkTextSecondary.withOpacity(0.3),
          ),
          const SizedBox(width: 6),
          Expanded(
            child: TextField(
              controller: _inputController,
              enabled: hasTerminals,
              style: const TextStyle(
                color: Color(0xFFD4D4DC),
                fontSize: 12,
                fontFamily: 'monospace',
              ),
              decoration: InputDecoration(
                hintText: hasTerminals
                    ? 'Enter command...'
                    : 'No active terminal',
                hintStyle: TextStyle(
                  color: AppColors.darkTextSecondary.withOpacity(0.3),
                  fontSize: 12,
                  fontFamily: 'monospace',
                ),
                border: InputBorder.none,
                isDense: true,
                contentPadding: const EdgeInsets.symmetric(
                  horizontal: 8,
                  vertical: 6,
                ),
              ),
              onSubmitted: hasTerminals
                  ? (value) => _handleSendCommand(provider, value)
                  : null,
            ),
          ),
          if (hasTerminals) ...[
            const SizedBox(width: 4),
            GestureDetector(
              onTap: () {
                if (_inputController.text.isNotEmpty) {
                  _handleSendCommand(provider, _inputController.text);
                }
              },
              child: MouseRegion(
                cursor: SystemMouseCursors.click,
                child: Container(
                  padding: const EdgeInsets.all(6),
                  decoration: BoxDecoration(
                    color: AppColors.darkAccent.withOpacity(0.15),
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Icon(
                    Icons.arrow_upward,
                    size: 14,
                    color: AppColors.darkAccent,
                  ),
                ),
              ),
            ),
          ],
        ],
      ),
    );
  }

  void _handleSpawnTerminal(AppProvider provider) {
    if (provider.activeProjectId.isEmpty) {
      provider.showToast(
        'Select a project first to spawn a terminal',
        type: 'error',
      );
      return;
    }
    provider.showToast(
      'Terminal session created',
      type: 'info',
    );
  }

  void _handleKillTerminal(AppProvider provider, String sessionId) {
    final term = provider.terminals.firstWhere(
      (t) => t.id == sessionId,
      orElse: () => TerminalSessionItem(id: '', name: ''),
    );
    if (term.id.isEmpty) return;

    provider.showToast(
      'Terminal ${term.name} closed',
      type: 'info',
    );
  }

  void _handleSendCommand(AppProvider provider, String command) {
    if (command.trim().isEmpty) return;
    _inputController.clear();
    _scrollToBottom();
  }

  void _handleClear(AppProvider provider) {
    setState(() {
      _logSearchQuery = '';
      _searchController.clear();
    });
    provider.showToast(
      'Terminal output cleared',
      type: 'info',
    );
  }
}

/// Pill toggle button for switching between log and terminal view modes.
class _ViewModePill extends StatelessWidget {
  final String label;
  final IconData icon;
  final bool isActive;
  final VoidCallback onTap;

  const _ViewModePill({
    required this.label,
    required this.icon,
    required this.isActive,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        decoration: BoxDecoration(
          color: isActive
              ? AppColors.darkAccent.withOpacity(0.15)
              : Colors.transparent,
          borderRadius: BorderRadius.circular(4),
          border: Border.all(
            color: isActive
                ? AppColors.darkAccent.withOpacity(0.4)
                : AppColors.darkBorder.withOpacity(0.2),
          ),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              icon,
              size: 12,
              color: isActive
                  ? AppColors.darkAccent
                  : AppColors.darkTextSecondary,
            ),
            const SizedBox(width: 4),
            Text(
              label,
              style: TextStyle(
                color: isActive
                    ? AppColors.darkAccent
                    : AppColors.darkTextSecondary,
                fontSize: 10,
                fontWeight: isActive ? FontWeight.w600 : FontWeight.normal,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
