import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/app_provider.dart';
import '../theme/app_theme.dart';

/// Cloudflare tunnel configuration panel mirroring CloudflareTunnel.tsx.
/// Shows tunnel mode selection (default vs token-based), configuration form,
/// save/cancel actions, and status notifications.
class CloudflareTunnelWidget extends StatefulWidget {
  const CloudflareTunnelWidget({super.key});

  @override
  State<CloudflareTunnelWidget> createState() =>
      _CloudflareTunnelWidgetState();
}

class _CloudflareTunnelWidgetState extends State<CloudflareTunnelWidget> {
  String _mode = 'default'; // 'default' or 'token'
  String _tunnelToken = '';
  bool _isLoading = true;
  bool _isSaving = false;
  String? _error;
  String? _successMsg;

  @override
  void initState() {
    super.initState();
    _loadConfig();
  }

  Future<void> _loadConfig() async {
    setState(() => _isLoading = true);
    // Config loading will be wired via provider/bridge in the future
    // For now, use default values matching the React fallback
    try {
      // Placeholder: in production this would call the bridge
      setState(() {
        _mode = 'default';
        _tunnelToken = '';
      });
    } catch (e) {
      debugPrint('No existing Cloudflare configuration, using defaults');
    } finally {
      setState(() => _isLoading = false);
    }
  }

  Future<void> _handleSave() async {
    setState(() {
      _isSaving = true;
      _error = null;
      _successMsg = null;
    });

    try {
      // In production, this would call:
      // await provider.rustBridge.saveCloudflareConfig({...});
      await Future.delayed(const Duration(milliseconds: 500));
      setState(() {
        _successMsg = 'Cloudflare configuration saved successfully!';
      });
      Future.delayed(const Duration(seconds: 3), () {
        if (mounted) {
          setState(() => _successMsg = null);
        }
      });
    } catch (e) {
      setState(() {
        _error = 'Failed to save configuration: $e';
      });
    } finally {
      setState(() => _isSaving = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_isLoading) {
      return Center(
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            SizedBox(
              width: 14,
              height: 14,
              child: CircularProgressIndicator(
                strokeWidth: 2,
                color: AppTheme.accentColor(context),
              ),
            ),
            const SizedBox(width: 8),
            Text('Loading Cloudflare Tunnel config...',
                style: TextStyle(
                    color: AppTheme.textSecondary(context),
                    fontSize: 11)),
          ],
        ),
      );
    }

    return Container(
      color: Theme.of(context).brightness == Brightness.dark
          ? const Color(0xFF14141E)
          : const Color(0xFFFAFAFC),
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: SizedBox(
          width: 680,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Title block
              Row(
                children: [
                  Icon(Icons.cloud_outlined,
                      size: 24, color: AppTheme.accentColor(context)),
                  const SizedBox(width: 12),
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('Cloudflare Tunnel Integration',
                          style: TextStyle(
                              color: AppTheme.textPrimary(context),
                              fontSize: 14,
                              fontWeight: FontWeight.w700)),
                      const SizedBox(height: 2),
                      Text('Share your local project to the internet',
                          style: TextStyle(
                              color: AppTheme.textSecondary(context),
                              fontSize: 11)),
                    ],
                  ),
                ],
              ),
              const SizedBox(height: 16),

              // Main configuration card
              Container(
                width: double.infinity,
                padding: const EdgeInsets.all(20),
                decoration: BoxDecoration(
                  color: Theme.of(context).brightness == Brightness.dark
                      ? AppColors.darkSurface
                      : AppColors.lightSurface,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(
                      color: AppTheme.borderPrimary(context)),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    // Mode selection
                    _buildModeSelector(),
                    const SizedBox(height: 20),

                    // Divider with content
                    Container(
                      padding: const EdgeInsets.only(top: 16),
                      decoration: BoxDecoration(
                        border: Border(
                          top: BorderSide(
                              color: AppTheme.borderPrimary(context)),
                        ),
                      ),
                      child: _mode == 'default'
                          ? _buildDefaultModeInfo(context)
                          : _buildTokenModeInput(context),
                    ),
                    const SizedBox(height: 16),

                    // Notifications
                    if (_error != null) _buildErrorBanner(),
                    if (_successMsg != null) _buildSuccessBanner(),
                    if (_error != null || _successMsg != null)
                      const SizedBox(height: 12),

                    // Actions
                    _buildActions(),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildModeSelector() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('OPERATION MODE',
            style: TextStyle(
                color: AppTheme.textSecondary(context),
                fontSize: 11,
                fontWeight: FontWeight.w700,
                letterSpacing: 0.5)),
        const SizedBox(height: 8),
        Row(
          children: [
            _buildRadioOption(
              value: 'default',
              label: 'Default (Tunnel Free)',
            ),
            const SizedBox(width: 24),
            _buildRadioOption(
              value: 'token',
              label: 'Use Cloudflare Tunnel Token',
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildRadioOption({
    required String value,
    required String label,
  }) {
    final isSelected = _mode == value;
    return GestureDetector(
      onTap: () => setState(() => _mode = value),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 14,
            height: 14,
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              border: Border.all(
                color: isSelected
                    ? AppTheme.accentColor(context)
                    : AppTheme.textSecondary(context),
                width: 1.5,
              ),
            ),
            child: isSelected
                ? Center(
                    child: Container(
                      width: 7,
                      height: 7,
                      decoration: BoxDecoration(
                        shape: BoxShape.circle,
                        color: AppTheme.accentColor(context),
                      ),
                    ),
                  )
                : null,
          ),
          const SizedBox(width: 8),
          Text(label,
              style: TextStyle(
                  color: AppTheme.textPrimary(context),
                  fontSize: 12)),
        ],
      ),
    );
  }

  Widget _buildDefaultModeInfo(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: AppTheme.accentColor(context).withOpacity(0.03),
        borderRadius: BorderRadius.circular(4),
        border: Border.all(
            color: AppTheme.accentColor(context).withOpacity(0.1)),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(Icons.help_outline,
              size: 16, color: AppTheme.accentColor(context)),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('Free Tunnel Mode (Recommended):',
                    style: TextStyle(
                        color: AppTheme.textPrimary(context),
                        fontSize: 11,
                        fontWeight: FontWeight.w600)),
                const SizedBox(height: 4),
                Text(
                  'You do not need a Cloudflare account. When starting a project with '
                  'Cloudflare Tunnel enabled, the system will automatically create a '
                  'random tunnel in the form of ',
                  style: TextStyle(
                      color: AppTheme.textSecondary(context),
                      fontSize: 11,
                      height: 1.5),
                ),
                Text(
                  'https://*.trycloudflare.com',
                  style: TextStyle(
                    color: AppTheme.accentColor(context),
                    fontSize: 11,
                    fontFamily: 'monospace',
                    backgroundColor:
                        Theme.of(context).brightness == Brightness.dark
                            ? AppColors.darkBg
                            : AppColors.lightBg,
                  ),
                ),
                Text(
                  ' linked directly to your project port.',
                  style: TextStyle(
                      color: AppTheme.textSecondary(context),
                      fontSize: 11,
                      height: 1.5),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildTokenModeInput(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Text('Tunnel Token (Base64)',
                style: TextStyle(
                    color: AppTheme.textSecondary(context),
                    fontSize: 11,
                    fontWeight: FontWeight.w600)),
            Text('Get from Cloudflare Zero Trust Dashboard',
                style: TextStyle(
                    color: AppTheme.textSecondary(context)
                        .withOpacity(0.6),
                    fontSize: 10)),
          ],
        ),
        const SizedBox(height: 6),
        TextField(
          obscureText: true,
          controller:
              TextEditingController(text: _tunnelToken),
          onChanged: (v) => _tunnelToken = v,
          style: TextStyle(
            color: AppTheme.textPrimary(context),
            fontSize: 11,
            fontFamily: 'monospace',
          ),
          decoration: InputDecoration(
            isDense: true,
            contentPadding: const EdgeInsets.symmetric(
                horizontal: 10, vertical: 8),
            hintText: 'Enter your Cloudflare Tunnel Token here...',
            hintStyle: TextStyle(
              color: AppTheme.textSecondary(context).withOpacity(0.4),
              fontSize: 10,
            ),
            filled: true,
            fillColor:
                Theme.of(context).brightness == Brightness.dark
                    ? AppColors.darkBg
                    : AppColors.lightBg,
            border: OutlineInputBorder(
              borderRadius: BorderRadius.circular(4),
              borderSide: BorderSide(
                color: AppTheme.borderPrimary(context),
                width: 0.5,
              ),
            ),
            enabledBorder: OutlineInputBorder(
              borderRadius: BorderRadius.circular(4),
              borderSide: BorderSide(
                color: AppTheme.borderPrimary(context),
                width: 0.5,
              ),
            ),
          ),
        ),
        const SizedBox(height: 6),
        Text(
          'Note: Named Tunnel in Token mode uses static routing '
          'configuration on your Cloudflare account to forward traffic.',
          style: TextStyle(
            color:
                AppTheme.textSecondary(context).withOpacity(0.6),
            fontSize: 10,
            height: 1.4,
          ),
        ),
      ],
    );
  }

  Widget _buildErrorBanner() {
    return Container(
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: AppColors.darkDanger.withOpacity(0.08),
        border: Border.all(
            color: AppColors.darkDanger.withOpacity(0.15)),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Row(
        children: [
          Icon(Icons.warning_amber_rounded,
              size: 14, color: AppColors.darkDanger),
          const SizedBox(width: 8),
          Expanded(
            child: Text(_error!,
                style: TextStyle(
                    color: AppColors.darkDanger, fontSize: 11)),
          ),
        ],
      ),
    );
  }

  Widget _buildSuccessBanner() {
    return Container(
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: AppColors.darkSuccess.withOpacity(0.08),
        border: Border.all(
            color: AppColors.darkSuccess.withOpacity(0.15)),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Row(
        children: [
          Icon(Icons.check_circle_outline,
              size: 14, color: AppColors.darkSuccess),
          const SizedBox(width: 8),
          Expanded(
            child: Text(_successMsg!,
                style: TextStyle(
                    color: AppColors.darkSuccess, fontSize: 11)),
          ),
        ],
      ),
    );
  }

  Widget _buildActions() {
    return Row(
      mainAxisAlignment: MainAxisAlignment.end,
      children: [
        GestureDetector(
          onTap: _isSaving ? null : _handleSave,
          child: Container(
            height: 30,
            padding: const EdgeInsets.symmetric(horizontal: 16),
            decoration: BoxDecoration(
              color: _isSaving
                  ? AppTheme.accentColor(context).withOpacity(0.5)
                  : AppTheme.accentColor(context),
              borderRadius: BorderRadius.circular(4),
            ),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(Icons.save_outlined,
                    size: 13,
                    color: Colors.white),
                const SizedBox(width: 6),
                Text(
                  _isSaving ? 'Saving...' : 'Save Configuration',
                  style: TextStyle(
                    color: Colors.white,
                    fontSize: 11,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ],
            ),
          ),
        ),
      ],
    );
  }
}
