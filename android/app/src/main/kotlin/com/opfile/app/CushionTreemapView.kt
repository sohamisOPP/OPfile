package com.opfile.app

import android.content.Context
import android.graphics.*
import android.os.Build
import android.os.VibrationEffect
import android.os.Vibrator
import android.util.AttributeSet
import android.view.MotionEvent
import android.view.View

data class TreemapNodeUi(
    val name: String,
    val path: String,
    val size: Long,
    val isDir: Boolean,
    val x: Float,
    val y: Float,
    val w: Float,
    val h: Float,
    val depth: Int
)

class CushionTreemapView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
    defStyleAttr: Int = 0
) : View(context, attrs, defStyleAttr) {

    private var nodes: List<TreemapNodeUi> = emptyList()
    private var selectedNode: TreemapNodeUi? = null
    var onNodeSelected: ((TreemapNodeUi) -> Unit)? = null

    // High-contrast AMOLED category palette
    private val palette = intArrayOf(
        Color.parseColor("#1E88E5"), // Blue
        Color.parseColor("#00897B"), // Teal
        Color.parseColor("#8E24AA"), // Purple
        Color.parseColor("#E64A19"), // Orange
        Color.parseColor("#3949AB"), // Indigo
        Color.parseColor("#FFA000"), // Amber
        Color.parseColor("#00ACC1"), // Cyan
        Color.parseColor("#D81B60"), // Pink
        Color.parseColor("#43A047"), // Green
        Color.parseColor("#6D4C41"), // Brown
        Color.parseColor("#546E7A"), // Slate
        Color.parseColor("#7B1FA2")  // Deep Purple
    )

    private val fillPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
    }
    private val strokePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        strokeWidth = 2.5f
        color = Color.BLACK
    }
    private val selectStrokePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        strokeWidth = 5f
        color = Color.WHITE
    }
    private val textPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.WHITE
        textSize = 28f
        typeface = Typeface.create(Typeface.DEFAULT, Typeface.BOLD)
        setShadowLayer(4f, 1f, 1f, Color.BLACK)
    }
    private val subTextPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#E0E0E0")
        textSize = 21f
        setShadowLayer(3f, 1f, 1f, Color.BLACK)
    }

    private var baseWidth = 1000f
    private var baseHeight = 1000f

    fun setNodes(newNodes: List<TreemapNodeUi>, refW: Float = 1000f, refH: Float = 1000f) {
        nodes = newNodes
        baseWidth = if (refW > 0) refW else 1000f
        baseHeight = if (refH > 0) refH else 1000f
        selectedNode = newNodes.firstOrNull()
        selectedNode?.let { onNodeSelected?.invoke(it) }
        invalidate()
    }

    fun setSelected(node: TreemapNodeUi?) {
        selectedNode = node
        invalidate()
    }

    fun getSelected(): TreemapNodeUi? = selectedNode

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        if (nodes.isEmpty() || width <= 0 || height <= 0) return

        val scaleX = width.toFloat() / baseWidth
        val scaleY = height.toFloat() / baseHeight

        for (i in nodes.indices) {
            val n = nodes[i]
            val left = n.x * scaleX
            val top = n.y * scaleY
            val right = (n.x + n.w) * scaleX
            val bottom = (n.y + n.h) * scaleY

            val rectW = right - left
            val rectH = bottom - top
            if (rectW < 1.5f || rectH < 1.5f) continue

            val baseColor = palette[(n.name.hashCode() and 0x7FFFFFFF) % palette.size]

            // 3D Cushion ridge surface gradient
            val shader = LinearGradient(
                left, top, right, bottom,
                intArrayOf(lighten(baseColor, 0.30f), baseColor, darken(baseColor, 0.40f)),
                floatArrayOf(0.0f, 0.45f, 1.0f),
                Shader.TileMode.CLAMP
            )
            fillPaint.shader = shader
            val rectF = RectF(left + 1f, top + 1f, right - 1f, bottom - 1f)
            canvas.drawRoundRect(rectF, 6f, 6f, fillPaint)
            fillPaint.shader = null

            // Outer gap divider border
            canvas.drawRoundRect(rectF, 6f, 6f, strokePaint)

            // Highlight border if selected
            if (selectedNode?.path == n.path) {
                canvas.drawRoundRect(rectF, 6f, 6f, selectStrokePaint)
            }

            // Text labels (fits within rectangle)
            if (rectW > 60f && rectH > 35f) {
                val maxChars = (rectW / 18f).toInt().coerceAtLeast(3)
                val label = if (n.name.length > maxChars) n.name.take(maxChars - 1) + "…" else n.name
                canvas.drawText(label, left + 8f, top + 30f, textPaint)

                if (rectH > 68f) {
                    val sizeStr = formatSize(n.size)
                    canvas.drawText(sizeStr, left + 8f, top + 56f, subTextPaint)
                }
            }
        }
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (event.action == MotionEvent.ACTION_UP) {
            performClick()
            val scaleX = width.toFloat() / baseWidth
            val scaleY = height.toFloat() / baseHeight
            val tx = event.x
            val ty = event.y

            for (n in nodes) {
                val left = n.x * scaleX
                val top = n.y * scaleY
                val right = (n.x + n.w) * scaleX
                val bottom = (n.y + n.h) * scaleY

                if (tx in left..right && ty in top..bottom) {
                    selectedNode = n
                    invalidate()
                    onNodeSelected?.invoke(n)
                    try {
                        val vibrator: Vibrator? = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                            val vm = context.getSystemService(Context.VIBRATOR_MANAGER_SERVICE) as? android.os.VibratorManager
                            vm?.defaultVibrator
                        } else {
                            @Suppress("DEPRECATION")
                            context.getSystemService(Context.VIBRATOR_SERVICE) as? Vibrator
                        }
                        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                            vibrator?.vibrate(VibrationEffect.createOneShot(12, VibrationEffect.DEFAULT_AMPLITUDE))
                        } else {
                            @Suppress("DEPRECATION")
                            vibrator?.vibrate(12)
                        }
                    } catch (_: Exception) {}
                    return true
                }
            }
        }
        return true
    }

    override fun performClick(): Boolean {
        super.performClick()
        return true
    }

    private fun lighten(color: Int, factor: Float): Int {
        val r = (Color.red(color) * (1f + factor)).coerceAtMost(255f).toInt()
        val g = (Color.green(color) * (1f + factor)).coerceAtMost(255f).toInt()
        val b = (Color.blue(color) * (1f + factor)).coerceAtMost(255f).toInt()
        return Color.rgb(r, g, b)
    }

    private fun darken(color: Int, factor: Float): Int {
        val r = (Color.red(color) * (1f - factor)).coerceAtLeast(0f).toInt()
        val g = (Color.green(color) * (1f - factor)).coerceAtLeast(0f).toInt()
        val b = (Color.blue(color) * (1f - factor)).coerceAtLeast(0f).toInt()
        return Color.rgb(r, g, b)
    }
}
