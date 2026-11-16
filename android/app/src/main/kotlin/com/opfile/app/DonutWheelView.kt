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

    var onSliceSelectedListener: ((WheelSliceItem) -> Unit)? = null
    var onFormatSelectedListener: ((String) -> Unit)? = null

    private var slices: List<WheelSliceItem> = emptyList()
    private var categoryTitle: String = "Convert"
    private var categoryCountText: String = ""

    private var currentRotation: Float = 0f
    private var lastTickIndex: Int = -1
    private var flingAnimator: ValueAnimator? = null

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
        textSize = 24f
        typeface = Typeface.create(Typeface.SANS_SERIF, Typeface.BOLD)
        textAlign = Paint.Align.CENTER
    }

    private val sliceSubPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#A0A0A0")
        textSize = 17f
        typeface = Typeface.create(Typeface.SANS_SERIF, Typeface.NORMAL)
        textAlign = Paint.Align.CENTER
    }

    private val centerTextPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.WHITE
        textSize = 36f
        typeface = Typeface.create(Typeface.SANS_SERIF, Typeface.BOLD)
        textAlign = Paint.Align.CENTER
    }

    private val centerSubPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#9E9E9E")
        textSize = 22f
        textAlign = Paint.Align.CENTER
    }

    private val ringRect = RectF()
    private val innerRect = RectF()

    private val gestureDetector = GestureDetector(context, object : GestureDetector.SimpleOnGestureListener() {
        override fun onDown(e: MotionEvent): Boolean {
            flingAnimator?.cancel()
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
            flingAnimator?.cancel()
            flingAnimator = ValueAnimator.ofFloat(currentRotation, currentRotation + dir * vel * 2f).apply {
                duration = 650
                interpolator = DecelerateInterpolator()
                addUpdateListener {
                    currentRotation = it.animatedValue as Float
                    checkHapticTick()
                    postInvalidateOnAnimation()
                }
            }
            flingAnimator?.start()
            return true
        }

        override fun onSingleTapUp(e: MotionEvent): Boolean {
            val cx = width / 2f
            val cy = height / 2f
            val dx = e.x - cx
            val dy = e.y - cy
            val dist = hypot(dx.toDouble(), dy.toDouble()).toFloat()
            val outerR = min(width, height) * 0.44f
            val innerR = outerR * 0.50f

            if (slices.isEmpty()) return false

            if (dist in innerR..outerR) {
                // Tapped directly on a sector: select that tapped sector
                val touchAngle = Math.toDegrees(atan2(dy.toDouble(), dx.toDouble())).toFloat()
                val sweep = 360f / slices.size
                val normalized = ((touchAngle - currentRotation) % 360f + 360f) % 360f
                val tappedIdx = (normalized / sweep).toInt().coerceIn(0, slices.size - 1)
                val slice = slices[tappedIdx]
                performHapticFeedback(HapticFeedbackConstants.CONTEXT_CLICK)
                dispatchSliceSelected(slice)
                return true
            } else if (dist < innerR) {
                // Tapped center circle: confirm the currently active sector
                val slice = getSelectedSlice() ?: slices[0]
                performHapticFeedback(HapticFeedbackConstants.CONTEXT_CLICK)
                dispatchSliceSelected(slice)
                return true
            }
            return false
        }
    })

    private fun dispatchSliceSelected(slice: WheelSliceItem) {
        onSliceSelectedListener?.invoke(slice)
        onFormatSelectedListener?.invoke(slice.targetFormat)
    }

    fun setWheelProfile(profile: WheelProfile) {
        flingAnimator?.cancel()
        categoryTitle = profile.name
        categoryCountText = profile.countText
        slices = profile.slices
        currentRotation = 0f
        lastTickIndex = -1
        invalidate()
    }

    fun setTargetFormats(list: List<String>) {
        flingAnimator?.cancel()
        categoryTitle = "Convert"
        categoryCountText = "${list.size} / 12"
        slices = list.map { WheelSliceItem(it.uppercase(), null, it) }
        currentRotation = 0f
        lastTickIndex = -1
        invalidate()
    }

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        flingAnimator?.cancel()
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
        if (slices.isEmpty()) return -1
        val sweep = 360f / slices.size
        // 270 degrees is top center (selection pointer)
        val normalized = ((-currentRotation + 270f) % 360f + 360f) % 360f
        return (normalized / sweep).toInt() % slices.size
    }

    fun getSelectedSlice(): WheelSliceItem? {
        val idx = getSelectedIndex()
        return if (idx in slices.indices) slices[idx] else null
    }

    fun getSelectedFormat(): String? {
        return getSelectedSlice()?.targetFormat
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
        if (slices.isEmpty()) return

        val cx = width / 2f
        val cy = height / 2f
        val outerR = min(width, height) * 0.44f
        val innerR = outerR * 0.50f

        ringRect.set(cx - outerR, cy - outerR, cx + outerR, cy + outerR)
        innerRect.set(cx - innerR, cy - innerR, cx + innerR, cy + innerR)

        val count = slices.size
        val sweep = 360f / count
        val selectedIdx = getSelectedIndex()

        // Draw sectors
        for (i in 0 until count) {
            val startAngle = currentRotation + i * sweep
            val isSelected = (i == selectedIdx)
            val isEven = (i % 2 == 0)

            sectorPaint.color = if (isSelected) Color.parseColor("#2E2E32")
                else (if (isEven) Color.parseColor("#1C1C1E") else Color.parseColor("#121212"))

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

            val slice = slices[i]
            val hasSubtitle = !slice.subtitle.isNullOrEmpty()

            textPaint.textSize = when {
                slice.label.length > 12 -> 17f
                slice.label.length > 8 -> 19f
                count > 10 -> 21f
                else -> 24f
            }
            textPaint.color = if (isSelected) Color.WHITE else Color.parseColor("#D0D0D0")

            val sub = slice.subtitle
            if (sub != null) {
                // If label has multiple words (e.g. "Trace to SVG"), handle neatly
                val labelLines = if (slice.label.contains(" ") && slice.label.length > 8) {
                    slice.label.split(" ", limit = 2)
                } else {
                    listOf(slice.label)
                }

                if (labelLines.size > 1) {
                    canvas.drawText(labelLines[0], tx, ty - 16f, textPaint)
                    canvas.drawText(labelLines[1], tx, ty + 2f, textPaint)
                    canvas.drawText(sub, tx, ty + 20f, sliceSubPaint)
                } else {
                    canvas.drawText(slice.label, tx, ty - 6f, textPaint)
                    canvas.drawText(sub, tx, ty + 15f, sliceSubPaint)
                }
            } else {
                val words = if (slice.label.contains(" ") && slice.label.length > 9) {
                    slice.label.split(" ", limit = 2)
                } else {
                    listOf(slice.label)
                }

                if (words.size > 1) {
                    val fm = textPaint.fontMetrics
                    val lineH = fm.descent - fm.ascent
                    canvas.drawText(words[0], tx, ty - lineH / 4f, textPaint)
                    canvas.drawText(words[1], tx, ty + lineH * 3f / 4f, textPaint)
                } else {
                    val fm = textPaint.fontMetrics
                    val baseline = ty - (fm.descent + fm.ascent) / 2f
                    canvas.drawText(slice.label, tx, baseline, textPaint)
                }
            }
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

        // Center text: Format label on top, action subtitle below (as it was originally)
        val selSlice = getSelectedSlice() ?: (if (slices.isNotEmpty()) slices[0] else null)
        val selLabel = selSlice?.label ?: ""
        val selFmt = selSlice?.targetFormat ?: ""
        val isCompress = selFmt.lowercase() in listOf("zip", "7z", "tar.gz", "tar.xz", "tar.bz2", "gz", "xz")
        val isExtract = selFmt.lowercase() == "extract" || selSlice?.actionType == "extract"

        centerTextPaint.textSize = if (selLabel.length > 8) 28f else 38f
        canvas.drawText(selLabel.uppercase(), cx, cy - 6f, centerTextPaint)

        val subText = when {
            isExtract -> "TAP TO EXTRACT"
            isCompress -> "TAP TO COMPRESS"
            selSlice?.isAction == true -> "TAP TO EXECUTE"
            else -> "TAP TO CONVERT"
        }
        canvas.drawText(subText, cx, cy + 30f, centerSubPaint)
    }
}
