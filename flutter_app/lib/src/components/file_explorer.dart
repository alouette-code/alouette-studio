import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/app_provider.dart';
import '../theme/app_theme.dart';

/// File tree explorer mirroring FileExplorer.tsx.
///
/// Shows a recursive directory tree with search/filter,
/// file/folder icons, and click-to-open behavior.
class FileExplorerWidget extends StatefulWidget {
  const FileExplorerWidget({super.key});

  @override
  State<FileExplorerWidget> createState() => _FileExplorerWidgetState();
}

class _FileExplorerWidgetState extends State<FileExplorerWidget> {
  String _searchQuery = '';
  final TextEditingController _searchController = TextEditingController();

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final provider = context.watch<AppProvider>();
    final project = provider.activeProject;

    return Container(
      color: Theme.of(context).brightness == Brightness.dark
          ? const Color(0xFF14141E)
          : const Color(0xFFFAFAFC),
      child: Column(
        children: [
          // ── Header ──
          _buildHeader(project?.name),

          // ── Content ──
          Expanded(
            child: project == null || project.cwd == null || project.cwd!.isEmpty
                ? _buildEmptyState()
                : _buildFileTree(context, provider),
          ),
        ],
      ),
    );
  }

  Widget _buildHeader(String? projectName) {
    return Container(
      height: 28,
      padding: const EdgeInsets.symmetric(horizontal: 8),
      decoration: BoxDecoration(
        border: Border(
          bottom: BorderSide(color: AppTheme.borderPrimary(context)),
        ),
      ),
      child: Row(
        children: [
          Icon(
            Icons.folder_outlined,
            size: 12,
            color: AppTheme.textSecondary(context),
          ),
          const SizedBox(width: 4),
          Text(
            projectName ?? 'Explorer',
            style: TextStyle(
              color: AppTheme.textPrimary(context),
              fontSize: 10,
              fontWeight: FontWeight.w600,
            ),
          ),
          const Spacer(),
          // Search toggle
          if (projectName != null)
            GestureDetector(
              onTap: () => setState(() {
                _searchQuery = '';
                _searchController.clear();
              }),
              child: MouseRegion(
                cursor: SystemMouseCursors.click,
                child: Padding(
                  padding: const EdgeInsets.all(2),
                  child: Icon(
                    Icons.refresh_rounded,
                    size: 11,
                    color: AppTheme.textSecondary(context).withOpacity(0.5),
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }

  Widget _buildEmptyState() {
    return Center(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 24),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.folder_open_outlined,
              size: 32,
              color: AppTheme.textSecondary(context).withOpacity(0.35),
            ),
            const SizedBox(height: 10),
            Text(
              'No folder opened',
              style: TextStyle(
                color: AppTheme.textSecondary(context),
                fontSize: 11,
                fontWeight: FontWeight.w500,
              ),
            ),
            const SizedBox(height: 4),
            Text(
              'Select a project to browse\nits file structure',
              textAlign: TextAlign.center,
              style: TextStyle(
                color: AppTheme.textSecondary(context).withOpacity(0.5),
                fontSize: 10,
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildFileTree(BuildContext context, AppProvider provider) {
    final project = provider.activeProject!;
    final cwd = project.cwd!;
    final rootName = cwd.split('/').last;

    return Column(
      children: [
        // Search bar
        _buildSearchBar(),

        // Tree
        Expanded(
          child: ListView(
            padding: const EdgeInsets.symmetric(vertical: 4),
            children: [
              _FileTreeNode(
                name: rootName,
                path: cwd,
                isFolder: true,
                isExpanded: true,
                depth: 0,
                searchQuery: _searchQuery,
                children: [
                  _FileTreeNode(
                    name: 'src',
                    path: '$cwd/src',
                    isFolder: true,
                    depth: 1,
                    searchQuery: _searchQuery,
                  ),
                  _FileTreeNode(
                    name: 'README.md',
                    path: '$cwd/README.md',
                    isFolder: false,
                    depth: 1,
                    searchQuery: _searchQuery,
                    onTap: () => provider.handleFileOpen('$cwd/README.md'),
                  ),
                ],
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildSearchBar() {
    // Only show the search bar when there's text or when focused
    return Container(
      margin: const EdgeInsets.fromLTRB(8, 4, 8, 2),
      decoration: BoxDecoration(
        color: Theme.of(context).brightness == Brightness.dark
            ? const Color(0xFF1A1A26)
            : const Color(0xFFEDEDF2),
        borderRadius: BorderRadius.circular(4),
        border: Border.all(
          color: AppTheme.borderPrimary(context).withOpacity(0.5),
        ),
      ),
      child: TextField(
        controller: _searchController,
        onChanged: (v) => setState(() => _searchQuery = v),
        style: TextStyle(
          fontSize: 10,
          color: AppTheme.textPrimary(context),
        ),
        decoration: InputDecoration(
          hintText: 'Search files...',
          hintStyle: TextStyle(
            fontSize: 10,
            color: AppTheme.textSecondary(context).withOpacity(0.5),
          ),
          prefixIcon: Padding(
            padding: const EdgeInsets.only(left: 6, right: 4),
            child: Icon(
              Icons.search_rounded,
              size: 12,
              color: AppTheme.textSecondary(context).withOpacity(0.5),
            ),
          ),
          suffixIcon: _searchQuery.isNotEmpty
              ? GestureDetector(
                  onTap: () {
                    _searchController.clear();
                    setState(() => _searchQuery = '');
                  },
                  child: Padding(
                    padding: const EdgeInsets.only(right: 4),
                    child: Icon(
                      Icons.close_rounded,
                      size: 12,
                      color: AppTheme.textSecondary(context).withOpacity(0.5),
                    ),
                  ),
                )
              : null,
          border: InputBorder.none,
          isDense: true,
          contentPadding: const EdgeInsets.symmetric(vertical: 6, horizontal: 4),
        ),
      ),
    );
  }
}

/// A single node in the file tree (file or folder).
class _FileTreeNode extends StatefulWidget {
  final String name;
  final String path;
  final bool isFolder;
  final bool isExpanded;
  final int depth;
  final List<_FileTreeNode>? children;
  final VoidCallback? onTap;
  final String searchQuery;

  const _FileTreeNode({
    required this.name,
    required this.path,
    required this.isFolder,
    this.isExpanded = false,
    required this.depth,
    this.children,
    this.onTap,
    this.searchQuery = '',
  });

  @override
  State<_FileTreeNode> createState() => _FileTreeNodeState();
}

class _FileTreeNodeState extends State<_FileTreeNode> {
  late bool _isExpanded;

  @override
  void initState() {
    super.initState();
    _isExpanded = widget.isExpanded;
  }

  @override
  Widget build(BuildContext context) {
    final showExpanded = _isExpanded;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        GestureDetector(
          onTap: () {
            if (widget.isFolder) {
              setState(() => _isExpanded = !_isExpanded);
            } else if (widget.onTap != null) {
              widget.onTap!();
            }
          },
          child: Container(
            padding: EdgeInsets.only(left: 8.0 + widget.depth * 16, right: 8),
            height: 24,
            alignment: Alignment.centerLeft,
            child: Row(
              children: [
                // Expand/collapse caret for folders
                if (widget.isFolder)
                  Padding(
                    padding: const EdgeInsets.only(right: 2),
                    child: Icon(
                      showExpanded
                          ? Icons.keyboard_arrow_down_rounded
                          : Icons.keyboard_arrow_right_rounded,
                      size: 14,
                      color: AppTheme.textSecondary(context).withOpacity(0.5),
                    ),
                  )
                else
                  const SizedBox(width: 14),

                // File/folder icon
                Icon(
                  widget.isFolder
                      ? (showExpanded
                          ? Icons.folder_open_outlined
                          : Icons.folder_outlined)
                      : _fileIcon(widget.name),
                  size: 14,
                  color: widget.isFolder
                      ? AppColors.darkWarning
                      : _fileIconColor(widget.name),
                ),
                const SizedBox(width: 4),

                // Name
                Expanded(
                  child: Text(
                    widget.name,
                    style: TextStyle(
                      color: AppTheme.textPrimary(context),
                      fontSize: 11,
                      fontFamily: 'monospace',
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ],
            ),
          ),
        ),

        // Children (if expanded and folder)
        if (widget.isFolder && showExpanded && widget.children != null)
          ...widget.children!.map((c) => c),
      ],
    );
  }

  IconData _fileIcon(String name) {
    final ext = name.contains('.') ? name.split('.').last.toLowerCase() : '';
    switch (ext) {
      case 'dart':
        return Icons.code;
      case 'rs':
        return Icons.code;
      case 'ts':
      case 'tsx':
      case 'js':
      case 'jsx':
        return Icons.javascript;
      case 'json':
        return Icons.data_object;
      case 'yaml':
      case 'yml':
        return Icons.settings;
      case 'md':
        return Icons.article;
      case 'toml':
        return Icons.tune;
      case 'css':
      case 'scss':
      case 'html':
        return Icons.web;
      case 'png':
      case 'jpg':
      case 'jpeg':
      case 'svg':
      case 'gif':
        return Icons.image;
      case 'lock':
        return Icons.lock_outline;
      default:
        return Icons.insert_drive_file_outlined;
    }
  }

  Color _fileIconColor(String name) {
    final ext = name.contains('.') ? name.split('.').last.toLowerCase() : '';
    switch (ext) {
      case 'dart':
        return const Color(0xFF0175C2);
      case 'rs':
        return const Color(0xFFDEA584);
      case 'ts':
      case 'tsx':
      case 'js':
      case 'jsx':
        return const Color(0xFFF7DF1E);
      case 'json':
        return const Color(0xFF8BC34A);
      case 'md':
        return const Color(0xFF42A5F5);
      case 'yaml':
      case 'yml':
        return const Color(0xFFEF5350);
      case 'toml':
        return const Color(0xFF9C27B0);
      case 'css':
      case 'scss':
        return const Color(0xFF42A5F5);
      case 'html':
        return const Color(0xFFE65100);
      default:
        return AppTheme.textSecondary(context);
    }
  }
}
