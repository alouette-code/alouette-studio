import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/app_provider.dart';
import '../models/project.dart';
import '../theme/app_theme.dart';

/// Project configuration form mirroring ConfigSetup.tsx.
/// Shows form fields for project name, command, args, CWD, port,
/// CPU/RAM limits, source, terminal mode, toolchain, auto-restart,
/// setup command/args, environment variables, and add/reset buttons.
class ConfigSetupWidget extends StatefulWidget {
  const ConfigSetupWidget({super.key});

  @override
  State<ConfigSetupWidget> createState() => _ConfigSetupWidgetState();
}

class _ConfigSetupWidgetState extends State<ConfigSetupWidget> {
  final _nameController = TextEditingController();
  final _commandController = TextEditingController();
  final _argsController = TextEditingController();
  final _cwdController = TextEditingController();
  final _portController = TextEditingController();
  final _cpuController = TextEditingController();
  final _ramController = TextEditingController();
  final _sourceController = TextEditingController();
  final _setupCmdController = TextEditingController();
  final _setupArgsController = TextEditingController();
  final _toolchainVersionController = TextEditingController();
  final _maxLogLinesController = TextEditingController();

  bool _autoRestart = false;
  String _terminalMode = 'log';
  String _toolchain = '';

  // Environment variables (key-value pairs)
  final List<_EnvEntry> _envEntries = [];

  bool _portInConflict = false;

  @override
  void dispose() {
    _nameController.dispose();
    _commandController.dispose();
    _argsController.dispose();
    _cwdController.dispose();
    _portController.dispose();
    _cpuController.dispose();
    _ramController.dispose();
    _sourceController.dispose();
    _setupCmdController.dispose();
    _setupArgsController.dispose();
    _toolchainVersionController.dispose();
    _maxLogLinesController.dispose();
    super.dispose();
  }

  void _resetForm() {
    setState(() {
      _nameController.clear();
      _commandController.clear();
      _argsController.clear();
      _cwdController.clear();
      _portController.clear();
      _cpuController.clear();
      _ramController.clear();
      _sourceController.clear();
      _setupCmdController.clear();
      _setupArgsController.clear();
      _toolchainVersionController.clear();
      _maxLogLinesController.clear();
      _autoRestart = false;
      _terminalMode = 'log';
      _toolchain = '';
      _envEntries.clear();
      _portInConflict = false;
    });
  }

  void _addEnvEntry() {
    setState(() {
      _envEntries.add(_EnvEntry(key: '', value: ''));
    });
  }

  void _removeEnvEntry(int index) {
    setState(() {
      _envEntries.removeAt(index);
    });
  }

  void _checkPortConflict() {
    final portText = _portController.text.trim();
    if (portText.isEmpty) {
      setState(() => _portInConflict = false);
      return;
    }
    final port = int.tryParse(portText);
    if (port == null) {
      setState(() => _portInConflict = false);
      return;
    }
    final provider = context.read<AppProvider>();
    final conflict = provider.projects.any((p) =>
        p.id != provider.activeProjectId && p.port == port);
    setState(() => _portInConflict = conflict);
  }

  void _handleAddProject() {
    final provider = context.read<AppProvider>();
    final env = <String, String>{};
    for (final entry in _envEntries) {
      if (entry.key.trim().isNotEmpty) {
        env[entry.key.trim()] = entry.value;
      }
    }

    // For now, this calls handleAddProject which will eventually
    // receive form data. In the future this will pass the config map.
    provider.handleAddProject();
  }

  @override
  Widget build(BuildContext context) {
    return Consumer<AppProvider>(
      builder: (context, provider, _) {
        return Container(
          color: Theme.of(context).brightness == Brightness.dark
              ? const Color(0xFF14141E)
              : const Color(0xFFFAFAFC),
          child: Column(
            children: [
              // Header
              Container(
                height: 28,
                padding: const EdgeInsets.symmetric(horizontal: 8),
                decoration: BoxDecoration(
                  border: Border(
                    bottom: BorderSide(
                        color: AppTheme.borderPrimary(context)),
                  ),
                ),
                child: Row(
                  children: [
                    Icon(Icons.settings_outlined,
                        size: 12,
                        color: AppTheme.textSecondary(context)),
                    const SizedBox(width: 4),
                    Text('Project Configuration',
                        style: TextStyle(
                            color: AppTheme.textPrimary(context),
                            fontSize: 10,
                            fontWeight: FontWeight.w600)),
                    const Spacer(),
                    Text('Setup Form',
                        style: TextStyle(
                            color: AppTheme.textSecondary(context),
                            fontSize: 9)),
                  ],
                ),
              ),
              // Scrollable form body
              Expanded(
                child: SingleChildScrollView(
                  padding: const EdgeInsets.all(12),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      // Row 1: Name + Auto-restart
                      _buildFormRow([
                        _buildField(
                          label: 'Tab / Project Identifier',
                          flex: 2,
                          controller: _nameController,
                          hint: 'e.g. Node backend service',
                        ),
                        _buildCheckboxField(
                          label: 'Auto Recovery',
                          flex: 1,
                          value: _autoRestart,
                          onChanged: (v) =>
                              setState(() => _autoRestart = v),
                          checkboxLabel: 'Auto restart',
                        ),
                      ]),
                      const SizedBox(height: 8),

                      // Row 2: Command + Args
                      _buildFormRow([
                        _buildField(
                          label: 'Executor',
                          flex: 1,
                          controller: _commandController,
                          hint: 'npm, node, ping',
                        ),
                        _buildField(
                          label: 'Command Arguments',
                          flex: 1,
                          controller: _argsController,
                          hint: 'run dev',
                        ),
                      ]),
                      const SizedBox(height: 8),

                      // CWD
                      _buildField(
                        label: 'Working Directory CWD (Optional)',
                        flex: 1,
                        controller: _cwdController,
                        hint: 'e.g. /home/project',
                      ),
                      const SizedBox(height: 8),

                      // Source
                      _buildField(
                        label: 'Clone / Copy Source (Optional)',
                        flex: 1,
                        controller: _sourceController,
                        hint: 'Git URL or local path to copy...',
                      ),
                      const SizedBox(height: 8),

                      // Setup command + args
                      _buildFormRow([
                        _buildField(
                          label: 'Setup Command',
                          flex: 1,
                          controller: _setupCmdController,
                          hint: 'npm install',
                        ),
                        _buildField(
                          label: 'Setup Args',
                          flex: 1,
                          controller: _setupArgsController,
                          hint: '--production',
                        ),
                      ]),
                      const SizedBox(height: 8),

                      // Toolchain select + version
                      _buildFormRow([
                        _buildDropdownField(
                          label: 'Toolchain (Proto)',
                          flex: 1,
                          value: _toolchain,
                          items: const [
                            DropdownMenuItem(
                                value: '',
                                child: Text(
                                    'System Default (No strict isolation)',
                                    style:
                                        TextStyle(fontSize: 11))),
                            DropdownMenuItem(
                                value: 'node',
                                child: Text('Node.js',
                                    style: TextStyle(fontSize: 11))),
                            DropdownMenuItem(
                                value: 'go',
                                child: Text('Go',
                                    style: TextStyle(fontSize: 11))),
                            DropdownMenuItem(
                                value: 'python',
                                child: Text('Python',
                                    style: TextStyle(fontSize: 11))),
                          ],
                          onChanged: (v) =>
                              setState(() => _toolchain = v ?? ''),
                        ),
                        if (_toolchain.isNotEmpty)
                          _buildField(
                            label: 'Version',
                            flex: 1,
                            controller: _toolchainVersionController,
                            hint: 'e.g. stable, 20.9.0',
                          ),
                      ]),
                      const SizedBox(height: 8),

                      // Terminal mode
                      _buildDropdownField(
                        label: 'Terminal Mode',
                        flex: 1,
                        value: _terminalMode,
                        items: const [
                          DropdownMenuItem(
                              value: 'log',
                              child: Text('Piped Log Stream (Mode B)',
                                  style: TextStyle(fontSize: 11))),
                          DropdownMenuItem(
                              value: 'pty',
                              child: Text('Interactive PTY (Mode A)',
                                  style: TextStyle(fontSize: 11))),
                        ],
                        onChanged: (v) =>
                            setState(() => _terminalMode = v ?? 'log'),
                      ),
                      const SizedBox(height: 8),

                      // Row: Port + CPU + RAM + Max Log Lines
                      _buildFormRow([
                        _buildField(
                          label: 'Scanner Port',
                          flex: 1,
                          controller: _portController,
                          hint: 'e.g. 3000',
                          isNumber: true,
                          onChanged: (_) => _checkPortConflict(),
                          suffix: _portInConflict
                              ? Icon(Icons.warning_amber_rounded,
                                  size: 14, color: AppColors.darkDanger)
                              : null,
                          conflictMsg: _portInConflict
                              ? 'Port conflict detected'
                              : null,
                        ),
                        _buildField(
                          label: 'Max CPU (%)',
                          flex: 1,
                          controller: _cpuController,
                          hint: 'No limit',
                          isNumber: true,
                        ),
                        _buildField(
                          label: 'Max RAM (MB)',
                          flex: 1,
                          controller: _ramController,
                          hint: 'No limit',
                          isNumber: true,
                        ),
                        _buildField(
                          label: 'Max Log Lines',
                          flex: 1,
                          controller: _maxLogLinesController,
                          hint: 'Default 5000',
                          isNumber: true,
                        ),
                      ]),
                      const SizedBox(height: 12),

                      // Environment Variables section
                      _buildEnvSection(),
                      const SizedBox(height: 12),

                      // Action buttons
                      _buildActionButtons(),
                    ],
                  ),
                ),
              ),
            ],
          ),
        );
      },
    );
  }

  Widget _buildFormRow(List<Widget> children) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: children
          .map((child) => Padding(
                padding: const EdgeInsets.only(right: 8),
                child: child,
              ))
          .toList(),
    );
  }

  Widget _buildField({
    required String label,
    required double flex,
    required TextEditingController controller,
    String? hint,
    bool isNumber = false,
    void Function(String)? onChanged,
    Widget? suffix,
    String? conflictMsg,
  }) {
    return Expanded(
      flex: flex ~/ 1,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            label,
            style: TextStyle(
              color: AppTheme.textSecondary(context),
              fontSize: 9,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: 4),
          TextField(
            controller: controller,
            onChanged: onChanged,
            keyboardType:
                isNumber ? TextInputType.number : TextInputType.text,
            style: TextStyle(
              color: AppTheme.textPrimary(context),
              fontSize: 11,
              fontFamily: 'monospace',
            ),
            decoration: InputDecoration(
              isDense: true,
              contentPadding:
                  const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
              hintText: hint ?? '',
              hintStyle: TextStyle(
                color: AppTheme.textSecondary(context).withOpacity(0.4),
                fontSize: 10,
              ),
              suffixIcon: suffix,
              filled: true,
              fillColor: Theme.of(context).brightness == Brightness.dark
                  ? AppColors.darkSurface
                  : AppColors.lightSurface,
              border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(4),
                borderSide: BorderSide(
                  color: conflictMsg != null
                      ? AppColors.darkDanger
                      : AppTheme.borderPrimary(context),
                  width: conflictMsg != null ? 1.5 : 0.5,
                ),
              ),
              enabledBorder: OutlineInputBorder(
                borderRadius: BorderRadius.circular(4),
                borderSide: BorderSide(
                  color: conflictMsg != null
                      ? AppColors.darkDanger
                      : AppTheme.borderPrimary(context),
                  width: conflictMsg != null ? 1.5 : 0.5,
                ),
              ),
              focusedBorder: OutlineInputBorder(
                borderRadius: BorderRadius.circular(4),
                borderSide: BorderSide(
                  color: conflictMsg != null
                      ? AppColors.darkDanger
                      : AppTheme.accentColor(context),
                  width: 1.5,
                ),
              ),
            ),
          ),
          if (conflictMsg != null)
            Padding(
              padding: const EdgeInsets.only(top: 2),
              child: Text(
                conflictMsg,
                style: const TextStyle(
                  color: AppColors.darkDanger,
                  fontSize: 9,
                ),
              ),
            ),
        ],
      ),
    );
  }

  Widget _buildCheckboxField({
    required String label,
    required double flex,
    required bool value,
    required ValueChanged<bool> onChanged,
    required String checkboxLabel,
  }) {
    return Expanded(
      flex: flex ~/ 1,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            label,
            style: TextStyle(
              color: AppTheme.textSecondary(context),
              fontSize: 9,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: 6),
          Row(
            children: [
              SizedBox(
                height: 16,
                width: 16,
                child: Checkbox(
                  value: value,
                  onChanged: (v) {
                    if (v != null) onChanged(v);
                  },
                  materialTapTargetSize:
                      MaterialTapTargetSize.shrinkWrap,
                  visualDensity: VisualDensity.compact,
                  side: BorderSide(
                    color: AppTheme.borderPrimary(context),
                    width: 1,
                  ),
                  fillColor: WidgetStateProperty.resolveWith((states) {
                    if (states.contains(WidgetState.selected)) {
                      return AppTheme.accentColor(context);
                    }
                    return Colors.transparent;
                  }),
                  checkColor: Colors.white,
                ),
              ),
              const SizedBox(width: 4),
              Text(
                checkboxLabel,
                style: TextStyle(
                  color: AppTheme.textPrimary(context),
                  fontSize: 10,
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildDropdownField({
    required String label,
    required double flex,
    required String value,
    required List<DropdownMenuItem<String>> items,
    required ValueChanged<String?> onChanged,
  }) {
    return Expanded(
      flex: flex ~/ 1,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            label,
            style: TextStyle(
              color: AppTheme.textSecondary(context),
              fontSize: 9,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: 4),
          Container(
            height: 30,
            decoration: BoxDecoration(
              color: Theme.of(context).brightness == Brightness.dark
                  ? AppColors.darkSurface
                  : AppColors.lightSurface,
              borderRadius: BorderRadius.circular(4),
              border: Border.all(
                color: AppTheme.borderPrimary(context),
                width: 0.5,
              ),
            ),
            padding: const EdgeInsets.symmetric(horizontal: 8),
            child: DropdownButtonHideUnderline(
              child: DropdownButton<String>(
                value: value,
                isExpanded: true,
                isDense: true,
                dropdownColor:
                    Theme.of(context).brightness == Brightness.dark
                        ? AppColors.darkSurface
                        : AppColors.lightSurface,
                style: TextStyle(
                  color: AppTheme.textPrimary(context),
                  fontSize: 11,
                ),
                items: items,
                onChanged: onChanged,
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildEnvSection() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Text(
              'Environment Variables',
              style: TextStyle(
                color: AppTheme.textPrimary(context),
                fontSize: 10,
                fontWeight: FontWeight.w600,
              ),
            ),
            GestureDetector(
              onTap: _addEnvEntry,
              child: Container(
                padding:
                    const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                decoration: BoxDecoration(
                  color: AppTheme.accentColor(context).withOpacity(0.15),
                  borderRadius: BorderRadius.circular(4),
                  border: Border.all(
                    color: AppTheme.accentColor(context).withOpacity(0.3),
                    width: 0.5,
                  ),
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(Icons.add,
                        size: 10,
                        color: AppTheme.accentColor(context)),
                    const SizedBox(width: 3),
                    Text(
                      'Add Variable',
                      style: TextStyle(
                        color: AppTheme.accentColor(context),
                        fontSize: 9,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
        const SizedBox(height: 6),
        if (_envEntries.isEmpty)
          Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(vertical: 16),
            decoration: BoxDecoration(
              border: Border.all(
                  color: AppTheme.borderPrimary(context), width: 0.5),
              borderRadius: BorderRadius.circular(4),
            ),
            child: Center(
              child: Text(
                'No environment variables defined',
                style: TextStyle(
                  color: AppTheme.textSecondary(context).withOpacity(0.5),
                  fontSize: 10,
                  fontStyle: FontStyle.italic,
                ),
              ),
            ),
          )
        else
          ..._envEntries.asMap().entries.map((entry) {
            final idx = entry.key;
            final env = entry.value;
            return Padding(
              padding: const EdgeInsets.only(bottom: 4),
              child: Row(
                children: [
                  Expanded(
                    child: TextField(
                      controller: TextEditingController(text: env.key),
                      onChanged: (v) => env.key = v,
                      style: TextStyle(
                        color: AppTheme.textPrimary(context),
                        fontSize: 10,
                        fontFamily: 'monospace',
                      ),
                      decoration: InputDecoration(
                        isDense: true,
                        contentPadding: const EdgeInsets.symmetric(
                            horizontal: 8, vertical: 5),
                        hintText: 'KEY',
                        hintStyle: TextStyle(
                          color: AppTheme.textSecondary(context)
                              .withOpacity(0.4),
                          fontSize: 10,
                        ),
                        filled: true,
                        fillColor:
                            Theme.of(context).brightness == Brightness.dark
                                ? AppColors.darkSurface
                                : AppColors.lightSurface,
                        border: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(4),
                          borderSide: BorderSide(
                            color: AppTheme.borderPrimary(context),
                            width: 0.5,
                          ),
                        ),
                      ),
                    ),
                  ),
                  const SizedBox(width: 4),
                  Expanded(
                    child: TextField(
                      controller:
                          TextEditingController(text: env.value),
                      onChanged: (v) => env.value = v,
                      style: TextStyle(
                        color: AppTheme.textPrimary(context),
                        fontSize: 10,
                        fontFamily: 'monospace',
                      ),
                      decoration: InputDecoration(
                        isDense: true,
                        contentPadding: const EdgeInsets.symmetric(
                            horizontal: 8, vertical: 5),
                        hintText: 'VALUE',
                        hintStyle: TextStyle(
                          color: AppTheme.textSecondary(context)
                              .withOpacity(0.4),
                          fontSize: 10,
                        ),
                        filled: true,
                        fillColor:
                            Theme.of(context).brightness == Brightness.dark
                                ? AppColors.darkSurface
                                : AppColors.lightSurface,
                        border: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(4),
                          borderSide: BorderSide(
                            color: AppTheme.borderPrimary(context),
                            width: 0.5,
                          ),
                        ),
                      ),
                    ),
                  ),
                  const SizedBox(width: 4),
                  GestureDetector(
                    onTap: () => _removeEnvEntry(idx),
                    child: Icon(Icons.remove_circle_outline,
                        size: 16, color: AppColors.darkDanger),
                  ),
                ],
              ),
            );
          }),
      ],
    );
  }

  Widget _buildActionButtons() {
    return Row(
      children: [
        GestureDetector(
          onTap: _resetForm,
          child: Container(
            padding:
                const EdgeInsets.symmetric(horizontal: 14, vertical: 7),
            decoration: BoxDecoration(
              color: AppTheme.textSecondary(context).withOpacity(0.1),
              borderRadius: BorderRadius.circular(4),
              border: Border.all(
                color: AppTheme.textSecondary(context).withOpacity(0.3),
                width: 0.5,
              ),
            ),
            child: Text(
              'New / Reset Form',
              style: TextStyle(
                color: AppTheme.textSecondary(context),
                fontSize: 10,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
        ),
        const Spacer(),
        GestureDetector(
          onTap: _handleAddProject,
          child: Container(
            padding:
                const EdgeInsets.symmetric(horizontal: 14, vertical: 7),
            decoration: BoxDecoration(
              color: AppTheme.accentColor(context),
              borderRadius: BorderRadius.circular(4),
            ),
            child: Text(
              'Save Tab Settings',
              style: TextStyle(
                color: Colors.white,
                fontSize: 10,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
        ),
      ],
    );
  }
}

/// Mutable helper class for environment variable entries.
class _EnvEntry {
  String key;
  String value;
  _EnvEntry({required this.key, required this.value});
}
