package com.opfile.app

import android.animation.ValueAnimator
import android.content.Context
import android.graphics.*
import android.os.Build
import android.os.VibrationEffect
import android.os.Vibrator
import android.util.AttributeSet
import android.view.GestureDetector
import android.view.HapticFeedbackConstants
import android.view.MotionEvent
import android.view.View
import android.view.animation.DecelerateInterpolator
import kotlin.math.*

class DonutWheelView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
    defStyleAttr: Int = 0
) : View(context, attrs, defStyleAttr) {

    var onFormatSelectedListener: ((String) -> Unit)? = null

    private var targets: List<String> = emptyList()
    private var currentRotation: Float = 0f
    private var lastTickIndex: Int = -1

    private val vibrator = context.getSystemService(Context.VIBRATOR_SERVICE) as? Vibrator

    private val sectorPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
    }

    private val sectorBorderPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        strokeWidth = 2f
        color = Color.parseColor("#222222")
    }

    private val textPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.WHITE
        textSize = 34f
        typeface = Typeface.create(Typeface.SANS_SERIF, Typeface.BOLD)
        textAlign = Paint.Align.CENTER
    }

    private val centerTextPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.WHITE
        textSize = 42f
        typeface = Typeface.create(Typeface.SANS_SERIF, Typeface.BOLD)
        textAlign = Paint.Align.CENTER
    }

    private val centerSubPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#9E9E9E")
        textSize = 24f
        textAlign = Paint.Align.CENTER
    }

    private val pointerPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.WHITE
        style = Paint.Style.FILL
    }

    private val ringRect = RectF()
    private val innerRect = RectF()

    private val gestureDetector = GestureDetector(context, object : GestureDetector.SimpleOnGestureListener() {
        override fun onDown(e: MotionEvent): Boolean {
            return true
        }

        override fun onScroll(
            e1: MotionEvent?,
            e2: MotionEvent,
            distanceX: Float,
            distanceY: Float
        ): Boolean {
            val cx = width / 2f
            val cy = height / 2f
            val angle1 = atan2(e2.y - cy + distanceY, e2.x - cx + distanceX)
            val angle2 = atan2(e2.y - cy, e2.x - cx)
            val diffDeg = Math.toDegrees((angle2 - angle1).toDouble()).toFloat()

            currentRotation += diffDeg
            checkHapticTick()
            postInvalidateOnAnimation()
            return true
        }

        override fun onFling(
            e1: MotionEvent?,
            e2: MotionEvent,
            velocityX: Float,
            velocityY: Float
        ): Boolean {
            val vel = (abs(velocityX) + abs(velocityY)) / 20f
            val dir = if (velocityX + velocityY > 0) 1f else -1f
            val animator = ValueAnimator.ofFloat(currentRotation, currentRotation + dir * vel * 2f).apply {
                duration = 650
                interpolator = DecelerateInterpolator()
                addUpdateListener {
                    currentRotation = it.animatedValue as Float
                    checkHapticTick()
                    postInvalidateOnAnimation()
                }
            }
            animator.start()
            return true
        }

        override fun onSingleTapUp(e: MotionEvent): Boolean {
            val cx = width / 2f
            val cy = height / 2f
            val dx = e.x - cx
            val dy = e.y - cy
            val dist = hypot(dx.toDouble(), dy.toDouble()).toFloat()
            val outerR = min(width, height) * 0.44f
            val innerR = outerR * 0.52f

            if (targets.isEmpty()) return false

            if (dist in innerR..outerR) {
                // Tapped directly on a sector: select that tapped sector format
                val touchAngle = Math.toDegrees(atan2(dy.toDouble(), dx.toDouble())).toFloat()
                val sweep = 360f / targets.size
                val normalized = ((touchAngle - currentRotation) % 360f + 360f) % 360f
                val tappedIdx = (normalized / sweep).toInt().coerceIn(0, targets.size - 1)
                val fmt = targets[tappedIdx]
                performHapticFeedback(HapticFeedbackConstants.CONTEXT_CLICK)
                onFormatSelectedListener?.invoke(fmt)
                return true
            } else if (dist < innerR) {
                // Tapped center circle: confirm the currently active format
                val fmt = getSelectedFormat() ?: targets[0]
                performHapticFeedback(HapticFeedbackConstants.CONTEXT_CLICK)
                onFormatSelectedListener?.invoke(fmt)
                return true
            }
            return false
        }
    })

    fun setTargetFormats(list: List<String>) {
        targets = list
        currentRotation = 0f
        lastTickIndex = -1
        invalidate()
    }

    private fun checkHapticTick() {
        val idx = getSelectedIndex()
        if (idx != -1 && idx != lastTickIndex) {
            lastTickIndex = idx
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                vibrator?.vibrate(VibrationEffect.createOneShot(12, VibrationEffect.DEFAULT_AMPLITUDE))
            } else {
                performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
            }
        }
    }

    private fun getSelectedIndex(): Int {
        if (targets.isEmpty()) return -1
        val sweep = 360f / targets.size
        // 270 degrees is top center (selection pointer)
        val normalized = ((-currentRotation + 270f) % 360f + 360f) % 360f
        return (normalized / sweep).toInt() % targets.size
    }

    fun getSelectedFormat(): String? {
        val idx = getSelectedIndex()
        return if (idx in targets.indices) targets[idx] else null
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        val handled = gestureDetector.onTouchEvent(event)
        if (event.action == MotionEvent.ACTION_UP) {
            performClick()
        }
        return true
    }

    override fun performClick(): Boolean {
        super.performClick()
        return true
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        if (targets.isEmpty()) return

        val cx = width / 2f
        val cy = height / 2f
        val outerR = min(width, height) * 0.44f
        val innerR = outerR * 0.52f

        ringRect.set(cx - outerR, cy - outerR, cx + outerR, cy + outerR)
        innerRect.set(cx - innerR, cy - innerR, cx + innerR, cy + innerR)

        val count = targets.size
        val sweep = 360f / count
        val selectedIdx = getSelectedIndex()

        // Draw sectors
        for (i in 0 until count) {
            val startAngle = currentRotation + i * sweep
            val isSelected = (i == selectedIdx)
            val isEven = (i % 2 == 0)

            sectorPaint.color = if (isSelected) Color.parseColor("#2E2E32") else (if (isEven) Color.parseColor("#1C1C1E") else Color.parseColor("#121212"))

            val path = Path()
            path.arcTo(ringRect, startAngle, sweep)
            path.arcTo(innerRect, startAngle + sweep, -sweep)
            path.close()
            canvas.drawPath(path, sectorPaint)

            if (isSelected) {
                sectorBorderPaint.color = Color.WHITE
                sectorBorderPaint.strokeWidth = 3f
            } else {
                sectorBorderPaint.color = Color.parseColor("#222222")
                sectorBorderPaint.strokeWidth = 1.5f
            }
            canvas.drawPath(path, sectorBorderPaint)

            // Text on sector: STRAIGHT, upright, horizontally centered, NEVER upside down
            val midAngleDeg = startAngle + sweep / 2f
            val midAngleRad = Math.toRadians(midAngleDeg.toDouble())
            val textR = (outerR + innerR) / 2f
            val tx = cx + (textR * cos(midAngleRad)).toFloat()
            val ty = cy + (textR * sin(midAngleRad)).toFloat()

            textPaint.textSize = if (count > 8) 22f else 26f
            textPaint.color = if (isSelected) Color.WHITE else Color.parseColor("#D0D0D0")
            val label = targets[i].uppercase()
            val fm = textPaint.fontMetrics
            val baseline = ty - (fm.descent + fm.ascent) / 2f
            canvas.drawText(label, tx, baseline, textPaint)
        }

        // Center circle (translucent glass AMOLED)
        val centerPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            color = Color.parseColor("#0A0A0A")
        }
        canvas.drawCircle(cx, cy, innerR - 4f, centerPaint)

        val centerBorder = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            style = Paint.Style.STROKE
            strokeWidth = 3f
            color = Color.WHITE
        }
        canvas.drawCircle(cx, cy, innerR - 4f, centerBorder)

        // Center selected format text
        val sel = getSelectedFormat() ?: (if (targets.isNotEmpty()) targets[0] else "")
        val isCompress = sel.lowercase() in listOf("zip", "7z", "tar.gz", "tar.xz", "tar.bz2", "gz", "xz")
        canvas.drawText(sel.uppercase(), cx, cy - 6f, centerTextPaint)
        canvas.drawText(if (isCompress) "TAP TO COMPRESS" else "TAP TO CONVERT", cx, cy + 30f, centerSubPaint)
    }
}
