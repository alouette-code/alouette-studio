import 'package:flutter/material.dart';
import '../theme/app_theme.dart';

/// A simple context menu widget that can be positioned at a given offset
class ContextMenu extends StatelessWidget {
  final Offset position;
  final List<ContextMenuItem> items;

  const ContextMenu({
    super.key,
    required this.position,
    required this.items,
  });

  @override
  Widget build(BuildContext context) {
    return Positioned(
      left: position.dx,
      top: position.dy,
      child: Material(
        color: Colors.transparent,
        child: Container(
          constraints: const BoxConstraints(minWidth: 160, maxWidth: 240),
          padding: const EdgeInsets.symmetric(vertical: 4),
          decoration: BoxDecoration(
            color: Theme.of(context).brightness == Brightness.dark
                ? const Color(0xFF1E1E2A)
                : Colors.white,
            borderRadius: BorderRadius.circular(8),
            border: Border.all(
              color: AppTheme.borderPrimary(context),
            ),
            boxShadow: [
              BoxShadow(
                color: Colors.black.withOpacity(0.3),
                blurRadius: 8,
                offset: const Offset(0, 4),
              ),
            ],
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: items.map((item) {
              return InkWell(
                onTap: item.onTap,
                hoverColor: AppTheme.hoverColor(context),
                child: Container(
                  width: double.infinity,
                  padding: const EdgeInsets.symmetric(
                    horizontal: 12,
                    vertical: 8,
                  ),
                  child: Text(
                    item.label,
                    style: TextStyle(
                      color: item.isDanger
                          ? AppColors.darkDanger
                          : AppTheme.textPrimary(context),
                      fontSize: 12,
                      fontWeight: item.isBold ? FontWeight.w600 : FontWeight.normal,
                    ),
                  ),
                ),
              );
            }).toList(),
          ),
        ),
      ),
    );
  }
}

class ContextMenuItem {
  final String label;
  final VoidCallback onTap;
  final bool isDanger;
  final bool isBold;

  ContextMenuItem({
    required this.label,
    required this.onTap,
    this.isDanger = false,
    this.isBold = false,
  });
}
