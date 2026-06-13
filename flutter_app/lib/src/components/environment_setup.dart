import 'dart:async';
import 'package:flutter/material.dart';
import '../models/project.dart';
import '../theme/app_theme.dart';

/// Environment simulation configuration panel mirroring EnvironmentSetup.tsx.
/// Shows firewall rules, network simulation (latency, jitter, packet loss),
/// unstable server simulation (drops, crashes, error codes),
/// and simulated resource limits (CPU/RAM).
class EnvironmentSetupWidget extends StatefulWidget {
  final Project? activeProject;

  const EnvironmentSetupWidget({
    super.key,
    this.activeProject,
  });

  @override
  State<EnvironmentSetupWidget> createState() =>
      _EnvironmentSetupWidgetState();
}

class _EnvSimulationConfig {
  String projectId = '';
  bool firewallEnabled = false;
  String firewallRules = '';
  bool weakNetworkEnabled = false;
  int latencyMs = 0;
  int jitterMs = 0;
  double lossRate = 0.0;
  int bandwidthKbps = 0;
  bool unstableServerEnabled = false;
  double unstableServerDropRate = 0.0;
  int unstableServerPeriodicCrashSecs = 0;
  double unstableServerErrorRate = 0.0;
  String unstableServerErrorCodes = '500,502,503';
  bool cpuLimitEnabled = false;
  int cpuLimitPercent = 80;
  bool ramLimitEnabled = false;
  int ramLimitMb = 2000;
}

class _EnvironmentSetupWidgetState
    extends State<EnvironmentSetupWidget> {
  _EnvSimulationConfig _config = _EnvSimulationConfig();
  bool _isLoading = false;
  bool _isSaving = false;
  String _saveStatus = 'idle'; // idle, saving, success, error
  Timer? _saveTimer;

  @override
  void initState() {
    super.initState();
    _loadConfig();
  }

  @override
  void dispose() {
    _saveTimer?.cancel();
    super.dispose();
  }

  Future<void> _loadConfig() async {
    setState(() => _isLoading = true);
    try {
      // In production, load from bridge:
      // final configs = await provider.rustBridge.loadEnvSimulationConfigs();
      await Future.delayed(const Duration(milliseconds: 200));
    } catch (e) {
      debugPrint('Failed to load env simulation configs: $e');
    } finally {
      setState(() => _isLoading = false);
    }
  }

  void _triggerSave() {
    _setSaveStatus('saving');
    _saveTimer?.cancel();
    _saveTimer = Timer(const Duration(milliseconds: 800), () async {
      try {
        // In production: await provider.rustBridge.saveEnvSimulationConfig({...});
        await Future.delayed(const Duration(milliseconds: 200));
        _setSaveStatus('success');
      } catch (e) {
        _setSaveStatus('error');
      } finally {
        if (mounted) setState(() => _isSaving = false);
      }
    });
  }

  void _setSaveStatus(String status) {
    if (mounted) {
      setState(() {
        _saveStatus = status;
        _isSaving = status == 'saving';
      });
    }
  }

  void _updateField<K>(void Function(_EnvSimulationConfig) updater) {
    setState(() {
      updater(_config);
    });
    _triggerSave();
  }

  void _applyFirewallPreset(String preset) {
    String rules;
    switch (preset) {
      case 'local_only':
        rules =
            '*.com, *.org, *.net, *.edu, *.gov, *.io, *.co, *.info, *.me, *.dev, *.ai, github.com, google.com';
        break;
      case 'block_social':
        rules =
            'facebook.com, *.facebook.com, twitter.com, *.twitter.com, x.com, *.x.com, instagram.com, *.instagram.com, tiktok.com, *.tiktok.com, youtube.com, *.youtube.com';
        break;
      case 'block_google':
        rules =
            'google.com, *.google.com, googleapis.com, *.googleapis.com, gstatic.com, *.gstatic.com';
        break;
      case 'clear':
      default:
        rules = '';
        break;
    }
    _updateField((c) => c.firewallRules = rules);
  }

  @override
  Widget build(BuildContext context) {
    if (_isLoading) {
      return Center(
        child: Text('Loading configuration...',
            style: TextStyle(
                color: AppTheme.textSecondary(context),
                fontSize: 11,
                fontFamily: 'monospace')),
      );
    }

    return Container(
      color: Theme.of(context).brightness == Brightness.dark
          ? const Color(0xFF14141E)
          : const Color(0xFFFAFAFC),
      child: Column(
        children: [
          // Header
          _buildHeader(context),
          // Body
          Expanded(
            child: SingleChildScrollView(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  // 2-column grid
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Expanded(
                        child: _buildSection(
                          context,
                          'Firewall Rules',
                          _config.firewallEnabled,
                          (v) => _updateField(
                              (c) => c.firewallEnabled = v),
                          _buildFirewallContent(context),
                        ),
                      ),
                      const SizedBox(width: 16),
                      Expanded(
                        child: _buildSection(
                          context,
                          'Network Simulation',
                          _config.weakNetworkEnabled,
                          (v) => _updateField(
                              (c) => c.weakNetworkEnabled = v),
                          _buildNetworkContent(context),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 16),
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Expanded(
                        child: _buildSection(
                          context,
                          'Unstable Server Simulation',
                          _config.unstableServerEnabled,
                          (v) => _updateField(
                              (c) => c.unstableServerEnabled = v),
                          _buildUnstableServerContent(context),
                        ),
                      ),
                      const SizedBox(width: 16),
                      Expanded(
                        child: _buildResourceLimitsContent(context),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildHeader(BuildContext context) {
    return Container(
      height: 28,
      padding: const EdgeInsets.symmetric(horizontal: 8),
      decoration: BoxDecoration(
        border: Border(
          bottom:
              BorderSide(color: AppTheme.borderPrimary(context)),
        ),
      ),
      child: Row(
        children: [
          Icon(Icons.science_outlined,
              size: 12, color: AppTheme.textSecondary(context)),
          const SizedBox(width: 4),
          Text('Environment Simulation Setup',
              style: TextStyle(
                  color: AppTheme.textPrimary(context),
                  fontSize: 10,
                  fontWeight: FontWeight.w600)),
          const Spacer(),
          // Save status indicator
          if (_isSaving)
            Text('Saving...',
                style: TextStyle(
                    color: AppTheme.textSecondary(context),
                    fontSize: 9))
          else if (_saveStatus == 'success')
            Text('Saved',
                style: TextStyle(
                    color: AppColors.darkSuccess, fontSize: 9))
          else if (_saveStatus == 'error')
            Text('Error saving',
                style: TextStyle(
                    color: AppColors.darkDanger, fontSize: 9))
          else
            Text('Auto-save',
                style: TextStyle(
                    color: AppTheme.textSecondary(context),
                    fontSize: 9)),
        ],
      ),
    );
  }

  Widget _buildSection(
    BuildContext context,
    String title,
    bool enabled,
    ValueChanged<bool> onToggle,
    Widget content,
  ) {
    return Container(
      decoration: BoxDecoration(
        border: Border.all(color: AppTheme.borderPrimary(context)),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Column(
        children: [
          // Section header with toggle
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              border: Border(
                bottom: BorderSide(
                    color: AppTheme.borderPrimary(context)),
              ),
            ),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text(title,
                    style: TextStyle(
                        color: AppTheme.textPrimary(context),
                        fontSize: 11,
                        fontWeight: FontWeight.w600)),
                SizedBox(
                  height: 16,
                  width: 16,
                  child: Checkbox(
                    value: enabled,
                    onChanged: (v) {
                      if (v != null) onToggle(v);
                    },
                    materialTapTargetSize:
                        MaterialTapTargetSize.shrinkWrap,
                    visualDensity: VisualDensity.compact,
                    side: BorderSide(
                      color: AppTheme.borderPrimary(context),
                      width: 1,
                    ),
                    fillColor:
                        WidgetStateProperty.resolveWith((states) {
                      if (states.contains(WidgetState.selected)) {
                        return AppTheme.accentColor(context);
                      }
                      return Colors.transparent;
                    }),
                    checkColor: Colors.white,
                  ),
                ),
              ],
            ),
          ),
          // Content (dimmed when disabled)
          Opacity(
            opacity: enabled ? 1.0 : 0.4,
            child: Padding(
              padding: const EdgeInsets.all(12),
              child: content,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildFirewallContent(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // Preset buttons
        Wrap(
          spacing: 6,
          runSpacing: 4,
          children: [
            _buildPresetButton(context, 'Local Only', () {
              if (_config.firewallEnabled)
                _applyFirewallPreset('local_only');
            }),
            _buildPresetButton(context, 'Block Socials', () {
              if (_config.firewallEnabled)
                _applyFirewallPreset('block_social');
            }),
            _buildPresetButton(context, 'Block Google', () {
              if (_config.firewallEnabled)
                _applyFirewallPreset('block_google');
            }),
            _buildPresetButton(
              context,
              'Clear',
              () {
                if (_config.firewallEnabled)
                  _applyFirewallPreset('clear');
              },
              danger: true,
            ),
          ],
        ),
        const SizedBox(height: 8),
        Text('Blocked domains/IPs (comma separated):',
            style: TextStyle(
                color: AppTheme.textSecondary(context), fontSize: 10)),
        const SizedBox(height: 4),
        TextField(
          enabled: _config.firewallEnabled,
          maxLines: 5,
          controller: TextEditingController(
              text: _config.firewallRules),
          onChanged: (v) =>
              _updateField((c) => c.firewallRules = v),
          style: TextStyle(
            color: AppTheme.textPrimary(context),
            fontSize: 11,
            fontFamily: 'monospace',
          ),
          decoration: _inputDecoration(
            hint: 'e.g. *.google.com, github.com, 1.1.1.1',
          ),
        ),
      ],
    );
  }

  Widget _buildNetworkContent(BuildContext context) {
    return Column(
      children: [
        _buildFieldRow(
          context,
          'Latency (ms):',
          TextField(
            enabled: _config.weakNetworkEnabled,
            controller: TextEditingController(
                text: _config.latencyMs.toString()),
            onChanged: (v) => _updateField((c) =>
                c.latencyMs = _parseInt(v, 0)),
            style: TextStyle(
                color: AppTheme.textPrimary(context), fontSize: 11, fontFamily: 'monospace'),
            decoration:
                _inputDecoration(),
            keyboardType: TextInputType.number,
          ),
        ),
        const SizedBox(height: 6),
        _buildFieldRow(
          context,
          'Jitter (ms):',
          TextField(
            enabled: _config.weakNetworkEnabled,
            controller: TextEditingController(
                text: _config.jitterMs.toString()),
            onChanged: (v) => _updateField(
                (c) => c.jitterMs = _parseInt(v, 0)),
            style: TextStyle(
                color: AppTheme.textPrimary(context), fontSize: 11, fontFamily: 'monospace'),
            decoration:
                _inputDecoration(),
            keyboardType: TextInputType.number,
          ),
        ),
        const SizedBox(height: 6),
        _buildFieldRow(
          context,
          'Packet Loss (%):',
          TextField(
            enabled: _config.weakNetworkEnabled,
            controller: TextEditingController(
                text: _config.lossRate.toString()),
            onChanged: (v) => _updateField(
                (c) => c.lossRate = _parseDouble(v, 0.0).clamp(0, 100)),
            style: TextStyle(
                color: AppTheme.textPrimary(context), fontSize: 11, fontFamily: 'monospace'),
            decoration:
                _inputDecoration(),
            keyboardType:
                const TextInputType.numberWithOptions(decimal: true),
          ),
        ),
        const SizedBox(height: 6),
        _buildFieldRow(
          context,
          'Bandwidth (Kbps, 0=unlimited):',
          TextField(
            enabled: _config.weakNetworkEnabled,
            controller: TextEditingController(
                text: _config.bandwidthKbps.toString()),
            onChanged: (v) => _updateField(
                (c) => c.bandwidthKbps = _parseInt(v, 0)),
            style: TextStyle(
                color: AppTheme.textPrimary(context), fontSize: 11, fontFamily: 'monospace'),
            decoration:
                _inputDecoration(),
            keyboardType: TextInputType.number,
          ),
        ),
      ],
    );
  }

  Widget _buildUnstableServerContent(BuildContext context) {
    return Column(
      children: [
        _buildFieldRow(
          context,
          'Drop Rate (%):',
          TextField(
            enabled: _config.unstableServerEnabled,
            controller: TextEditingController(
                text: _config.unstableServerDropRate.toString()),
            onChanged: (v) => _updateField((c) =>
                c.unstableServerDropRate =
                    _parseDouble(v, 0.0).clamp(0, 100)),
            style: TextStyle(
                color: AppTheme.textPrimary(context), fontSize: 11, fontFamily: 'monospace'),
            decoration:
                _inputDecoration(),
            keyboardType:
                const TextInputType.numberWithOptions(decimal: true),
          ),
        ),
        const SizedBox(height: 6),
        _buildFieldRow(
          context,
          'Error Rate (%):',
          TextField(
            enabled: _config.unstableServerEnabled,
            controller: TextEditingController(
                text: _config.unstableServerErrorRate.toString()),
            onChanged: (v) => _updateField((c) =>
                c.unstableServerErrorRate =
                    _parseDouble(v, 0.0).clamp(0, 100)),
            style: TextStyle(
                color: AppTheme.textPrimary(context), fontSize: 11, fontFamily: 'monospace'),
            decoration:
                _inputDecoration(),
            keyboardType:
                const TextInputType.numberWithOptions(decimal: true),
          ),
        ),
        const SizedBox(height: 6),
        _buildFieldRow(
          context,
          'Periodic Crash (s, 0=disable):',
          TextField(
            enabled: _config.unstableServerEnabled,
            controller: TextEditingController(
                text: _config.unstableServerPeriodicCrashSecs
                    .toString()),
            onChanged: (v) => _updateField((c) =>
                c.unstableServerPeriodicCrashSecs =
                    _parseInt(v, 0)),
            style: TextStyle(
                color: AppTheme.textPrimary(context), fontSize: 11, fontFamily: 'monospace'),
            decoration:
                _inputDecoration(),
            keyboardType: TextInputType.number,
          ),
        ),
        const SizedBox(height: 6),
        _buildFieldRow(
          context,
          'API Error Codes (CSV):',
          TextField(
            enabled: _config.unstableServerEnabled,
            controller: TextEditingController(
                text: _config.unstableServerErrorCodes),
            onChanged: (v) => _updateField(
                (c) => c.unstableServerErrorCodes = v),
            style: TextStyle(
                color: AppTheme.textPrimary(context), fontSize: 11, fontFamily: 'monospace'),
            decoration:
                _inputDecoration(),
          ),
        ),
      ],
    );
  }

  Widget _buildResourceLimitsContent(BuildContext context) {
    return Container(
      decoration: BoxDecoration(
        border:
            Border.all(color: AppTheme.borderPrimary(context)),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Column(
        children: [
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              border: Border(
                bottom: BorderSide(
                    color: AppTheme.borderPrimary(context)),
              ),
            ),
            child: Text('Simulated Resource Limits',
                style: TextStyle(
                    color: AppTheme.textPrimary(context),
                    fontSize: 11,
                    fontWeight: FontWeight.w600)),
          ),
          Padding(
            padding: const EdgeInsets.all(12),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'These limits are for sandbox simulation only and are independent of system project configurations.',
                  style: TextStyle(
                    color: AppTheme.textSecondary(context),
                    fontSize: 10,
                  ),
                ),
                const SizedBox(height: 12),
                // CPU Limit
                Row(
                  children: [
                    SizedBox(
                      height: 16,
                      width: 16,
                      child: Checkbox(
                        value: _config.cpuLimitEnabled,
                        onChanged: (v) => _updateField(
                            (c) => c.cpuLimitEnabled = v!),
                        materialTapTargetSize:
                            MaterialTapTargetSize.shrinkWrap,
                        visualDensity: VisualDensity.compact,
                        side: BorderSide(
                          color: AppTheme.borderPrimary(context),
                          width: 1,
                        ),
                        fillColor: WidgetStateProperty.resolveWith(
                            (states) {
                          if (states
                              .contains(WidgetState.selected)) {
                            return AppTheme.accentColor(context);
                          }
                          return Colors.transparent;
                        }),
                        checkColor: Colors.white,
                      ),
                    ),
                    const SizedBox(width: 8),
                    Text('CPU Limit (%):',
                        style: TextStyle(
                            color: AppTheme.textPrimary(context),
                            fontSize: 11)),
                    const Spacer(),
                    SizedBox(
                      width: 80,
                      child: TextField(
                        enabled: _config.cpuLimitEnabled,
                        controller: TextEditingController(
                            text: _config.cpuLimitPercent
                                .toString()),
                        onChanged: (v) => _updateField((c) =>
                            c.cpuLimitPercent =
                                _parseInt(v, 1).clamp(1, 100)),
                        style: TextStyle(
                            color: AppTheme.textPrimary(context),
                            fontSize: 11,
                            fontFamily: 'monospace'),
                        decoration: _inputDecoration(),
                        keyboardType: TextInputType.number,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                // RAM Limit
                Row(
                  children: [
                    SizedBox(
                      height: 16,
                      width: 16,
                      child: Checkbox(
                        value: _config.ramLimitEnabled,
                        onChanged: (v) => _updateField(
                            (c) => c.ramLimitEnabled = v!),
                        materialTapTargetSize:
                            MaterialTapTargetSize.shrinkWrap,
                        visualDensity: VisualDensity.compact,
                        side: BorderSide(
                          color: AppTheme.borderPrimary(context),
                          width: 1,
                        ),
                        fillColor: WidgetStateProperty.resolveWith(
                            (states) {
                          if (states
                              .contains(WidgetState.selected)) {
                            return AppTheme.accentColor(context);
                          }
                          return Colors.transparent;
                        }),
                        checkColor: Colors.white,
                      ),
                    ),
                    const SizedBox(width: 8),
                    Text('RAM Limit (MB):',
                        style: TextStyle(
                            color: AppTheme.textPrimary(context),
                            fontSize: 11)),
                    const Spacer(),
                    SizedBox(
                      width: 80,
                      child: TextField(
                        enabled: _config.ramLimitEnabled,
                        controller: TextEditingController(
                            text:
                                _config.ramLimitMb.toString()),
                        onChanged: (v) => _updateField(
                            (c) => c.ramLimitMb = _parseInt(v, 1)),
                        style: TextStyle(
                            color: AppTheme.textPrimary(context),
                            fontSize: 11,
                            fontFamily: 'monospace'),
                        decoration: _inputDecoration(),
                        keyboardType: TextInputType.number,
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildFieldRow(
      BuildContext context, String label, Widget field) {
    return Row(
      children: [
        SizedBox(
          width: 130,
          child: Text(label,
              style: TextStyle(
                  color: AppTheme.textSecondary(context),
                  fontSize: 10)),
        ),
        const SizedBox(width: 4),
        Expanded(child: field),
      ],
    );
  }

  Widget _buildPresetButton(
      BuildContext context, String label, VoidCallback onPressed,
      {bool danger = false}) {
    return GestureDetector(
      onTap: onPressed,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
        decoration: BoxDecoration(
          color: danger
              ? AppColors.darkDanger.withOpacity(0.1)
              : Theme.of(context).brightness == Brightness.dark
                  ? AppColors.darkSurface
                  : AppColors.lightSurface,
          borderRadius: BorderRadius.circular(2),
          border: Border.all(
            color: danger
                ? AppColors.darkDanger
                : AppTheme.borderPrimary(context),
            width: 0.5,
          ),
        ),
        child: Text(
          label,
          style: TextStyle(
            color: danger
                ? AppColors.darkDanger
                : AppTheme.textPrimary(context),
            fontSize: 9,
          ),
        ),
      ),
    );
  }

  InputDecoration _inputDecoration({String? hint}) {
    return InputDecoration(
      isDense: true,
      contentPadding:
          const EdgeInsets.symmetric(horizontal: 8, vertical: 5),
      hintText: hint,
      hintStyle: TextStyle(
        color: AppTheme.textSecondary(context).withOpacity(0.4),
        fontSize: 10,
      ),
      filled: true,
      fillColor: Theme.of(context).brightness == Brightness.dark
          ? AppColors.darkSurface
          : AppColors.lightSurface,
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(3),
        borderSide: BorderSide(
          color: AppTheme.borderPrimary(context),
          width: 0.5,
        ),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(3),
        borderSide: BorderSide(
          color: AppTheme.borderPrimary(context),
          width: 0.5,
        ),
      ),
    );
  }

  int _parseInt(String v, int fallback) {
    final parsed = int.tryParse(v);
    return parsed != null && parsed >= 0 ? parsed : fallback;
  }

  double _parseDouble(String v, double fallback) {
    final parsed = double.tryParse(v);
    return parsed != null && parsed >= 0 ? parsed : fallback;
  }
}
