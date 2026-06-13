import 'package:flutter/material.dart';
import '../theme/app_theme.dart';

/// Horizontal resize handle (draggable divider)
class HorizontalResizeHandle extends StatefulWidget {
  final void Function(double delta) onResize;
  const HorizontalResizeHandle({super.key, required this.onResize});

  @override
  State<HorizontalResizeHandle> createState() => _HorizontalResizeHandleState();
}

class _HorizontalResizeHandleState extends State<HorizontalResizeHandle> {
  bool _isDragging = false;

  void _onDragStart(DragStartDetails details) => setState(() => _isDragging = true);
  void _onDragUpdate(DragUpdateDetails details) => widget.onResize(details.delta.dy);
  void _onDragEnd(DragEndDetails details) => setState(() => _isDragging = false);

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onHorizontalDragStart: _onDragStart,
      onHorizontalDragUpdate: _onDragUpdate,
      onHorizontalDragEnd: _onDragEnd,
      child: Container(
        height: 4,
        margin: const EdgeInsets.symmetric(horizontal: 0),
        decoration: BoxDecoration(
          color: _isDragging
              ? AppTheme.accentColor(context).withOpacity(0.3)
              : Colors.transparent,
          borderRadius: BorderRadius.circular(2),
        ),
        child: Row(
          children: [
            Expanded(
              child: Container(
                height: 2,
                margin: const EdgeInsets.symmetric(horizontal: 40),
                decoration: BoxDecoration(
                  color: _isDragging
                      ? AppTheme.accentColor(context)
                      : AppTheme.borderPrimary(context).withOpacity(0.3),
                  borderRadius: BorderRadius.circular(1),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Vertical resize handle (draggable divider between columns)
class VerticalResizeHandle extends StatefulWidget {
  final void Function(double delta) onResize;
  const VerticalResizeHandle({super.key, required this.onResize});

  @override
  State<VerticalResizeHandle> createState() => _VerticalResizeHandleState();
}

class _VerticalResizeHandleState extends State<VerticalResizeHandle> {
  bool _isDragging = false;

  void _onDragStart(DragStartDetails details) => setState(() => _isDragging = true);
  void _onDragUpdate(DragUpdateDetails details) => widget.onResize(details.delta.dx);
  void _onDragEnd(DragEndDetails details) => setState(() => _isDragging = false);

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onVerticalDragStart: _onDragStart,
      onVerticalDragUpdate: _onDragUpdate,
      onVerticalDragEnd: _onDragEnd,
      child: MouseRegion(
        cursor: SystemMouseCursors.resizeColumn,
        child: Container(
          width: 4,
          color: _isDragging
              ? AppTheme.accentColor(context).withOpacity(0.3)
              : Colors.transparent,
        ),
      ),
    );
  }
}
