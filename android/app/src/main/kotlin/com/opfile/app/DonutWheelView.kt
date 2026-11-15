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
        color = Color.parseColor("#6C5CE7")
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
        color = Color.parseColor("#6C5CE7")
        style = Paint.Style.FILL
    }

    private val ringRect = RectF()
    private val innerRect = RectF()

    private val gestureDetector = GestureDetector(context, object : GestureDetector.SimpleOnGestureListener() {
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
            val outerR = min(width, height) * 0.45f
            val innerR = outerR * 0.55f

            if (dist in innerR..outerR) {
                // Tapped on the donut ring - select current top format
                getSelectedFormat()?.let { fmt ->
                    performHapticFeedback(HapticFeedbackConstants.CONTEXT_CLICK)
                    onFormatSelectedListener?.invoke(fmt)
                }
                return true
            } else if (dist < innerR) {
                // Tapped center - confirm selection
                getSelectedFormat()?.let { fmt ->
                    performHapticFeedback(HapticFeedbackConstants.CONTEXT_CLICK)
                    onFormatSelectedListener?.invoke(fmt)
                }
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
        return gestureDetector.onTouchEvent(event) || super.onTouchEvent(event)
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

        // Draw sectors
        for (i in 0 until count) {
            val startAngle = currentRotation + i * sweep
            val isEven = (i % 2 == 0)
            sectorPaint.color = if (isEven) Color.parseColor("#1C1C1E") else Color.parseColor("#141414")

            val path = Path()
            path.arcTo(ringRect, startAngle, sweep)
            path.arcTo(innerRect, startAngle + sweep, -sweep)
            path.close()
            canvas.drawPath(path, sectorPaint)
            canvas.drawPath(path, sectorBorderPaint)

            // Text on sector
            val midAngleRad = Math.toRadians((startAngle + sweep / 2f).toDouble())
            val textR = (outerR + innerR) / 2f
            val tx = cx + (textR * cos(midAngleRad)).toFloat()
            val ty = cy + (textR * sin(midAngleRad)).toFloat() + 10f

            canvas.drawText(targets[i].uppercase(), tx, ty, textPaint)
        }

        // Draw top indicator pointer
        val pointerPath = Path().apply {
            moveTo(cx, cy - outerR - 10f)
            lineTo(cx - 16f, cy - outerR + 20f)
            lineTo(cx + 16f, cy - outerR + 20f)
            close()
        }
        canvas.drawPath(pointerPath, pointerPaint)

        // Center circle (translucent glass AMOLED)
        val centerPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            color = Color.parseColor("#0A0A0A")
        }
        canvas.drawCircle(cx, cy, innerR - 4f, centerPaint)

        val centerBorder = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            style = Paint.Style.STROKE
            strokeWidth = 3f
            color = Color.parseColor("#6C5CE7")
        }
        canvas.drawCircle(cx, cy, innerR - 4f, centerBorder)

        // Center selected format text
        val sel = getSelectedFormat() ?: ""
        canvas.drawText(sel.uppercase(), cx, cy - 6f, centerTextPaint)
        canvas.drawText("TAP TO CONVERT", cx, cy + 30f, centerSubPaint)
    }
}
