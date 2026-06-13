import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/app_provider.dart';
import '../theme/app_theme.dart';

class SqliteEditorWidget extends StatelessWidget {
  final String filePath;
  const SqliteEditorWidget({super.key, required this.filePath});

  @override
  Widget build(BuildContext context) {
    return Container(
      color: Theme.of(context).brightness == Brightness.dark
          ? const Color(0xFF14141E)
          : const Color(0xFFFAFAFC),
      child: Column(
        children: [
          // Header
          Container(
            height: 36,
            padding: const EdgeInsets.symmetric(horizontal: 12),
            decoration: BoxDecoration(
              border: Border(
                bottom: BorderSide(color: AppTheme.borderPrimary(context)),
              ),
            ),
            child: Row(
              children: [
                Icon(Icons.storage_rounded, size: 14, color: AppColors.darkInfo),
                const SizedBox(width: 8),
                Text(
                  filePath.split('/').last,
                  style: TextStyle(color: AppTheme.textPrimary(context), fontSize: 12, fontWeight: FontWeight.w600),
                ),
                const SizedBox(width: 8),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
                  decoration: BoxDecoration(
                    color: AppColors.darkInfo.withOpacity(0.2),
                    borderRadius: BorderRadius.circular(3),
                  ),
                  child: Text('SQLite', style: TextStyle(color: AppColors.darkInfo, fontSize: 9, fontWeight: FontWeight.w600)),
                ),
                const Spacer(),
                Text(
                  filePath,
                  style: TextStyle(color: AppTheme.textSecondary(context), fontSize: 9, fontFamily: 'monospace'),
                ),
              ],
            ),
          ),
          // Body - placeholder for now
          Expanded(
            child: Center(
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(Icons.table_chart_outlined, size: 32, color: AppTheme.textSecondary(context).withOpacity(0.4)),
                  const SizedBox(height: 12),
                  Text('SQLite Database Browser', style: TextStyle(color: AppTheme.textPrimary(context), fontSize: 13)),
                  const SizedBox(height: 4),
                  Text('FRB integration to query tables coming soon', style: TextStyle(color: AppTheme.textSecondary(context), fontSize: 11)),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}
