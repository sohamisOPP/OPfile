package com.opfile.app

data class WheelSliceItem(
    val label: String,
    val subtitle: String? = null,
    val targetFormat: String,
    val isAction: Boolean = false,
    val actionType: String = ""
)

data class WheelProfile(
    val name: String,
    val countText: String,
    val extensions: Set<String>,
    val slices: List<WheelSliceItem>
)

object WheelProfiles {

    val PROFILES: List<WheelProfile> = listOf(
        // 1. Images
        WheelProfile(
            name = "Images",
            countText = "10 / 12",
            extensions = setOf(
                "png", "jpg", "jpeg", "jpe", "jfif", "webp", "tiff", "tif", "bmp", "dib",
                "gif", "ico", "avif", "heic", "heif", "hif", "jxl", "jp2", "j2k", "jpf",
                "jpx", "psd", "psb", "tga", "icb", "vda", "vst", "ppm", "pgm", "pbm",
                "pnm", "icns", "apng", "mng", "qoi", "dds", "pcx", "cur", "xpm", "xbm",
                "wbmp", "pam", "pfm", "sgi", "rgb", "rgba", "bw", "ras", "sun", "miff",
                "dcm", "dicom", "pcd", "exr", "hdr", "xcf", "dng", "cr2", "cr3", "crw",
                "nef", "nrw", "arw", "srf", "sr2", "orf", "rw2", "raf", "pef", "dcr",
                "kdc", "erf", "mos", "raw"
            ),
            slices = listOf(
                WheelSliceItem("PNG", null, "png"),
                WheelSliceItem("JPG", null, "jpg"),
                WheelSliceItem("WEBP", null, "webp"),
                WheelSliceItem("PDF", null, "pdf"),
                WheelSliceItem("Trace to SVG", "SVG", "svg"),
                WheelSliceItem("Line art to SVG", "SVG", "svg"),
                WheelSliceItem("TIFF", null, "tiff"),
                WheelSliceItem("BMP", null, "bmp"),
                WheelSliceItem("ICO", null, "ico"),
                WheelSliceItem("Half size", "Same for...", "half_size", isAction = true, actionType = "half_size")
            )
        ),

        // 2. PDF
        WheelProfile(
            name = "PDF",
            countText = "12 / 12",
            extensions = setOf("pdf"),
            slices = listOf(
                WheelSliceItem("PNG", null, "png"),
                WheelSliceItem("JPG", null, "jpg"),
                WheelSliceItem("Long image", "PNG", "png", isAction = true, actionType = "long_image"),
                WheelSliceItem("DOCX", null, "docx"),
                WheelSliceItem("PPTX (slides)", "PPTX", "pptx"),
                WheelSliceItem("TXT", null, "txt"),
                WheelSliceItem("Markdown", null, "md"),
                WheelSliceItem("EPUB", null, "epub"),
                WheelSliceItem("HWPX", null, "hwpx"),
                WheelSliceItem("HWP", null, "hwp"),
                WheelSliceItem("Split PDF", "PDF", "split_pdf", isAction = true, actionType = "split_pdf"),
                WheelSliceItem("Merge PDF", "PDF", "merge_pdf", isAction = true, actionType = "merge_pdf")
            )
        ),

        // 3. Documents
        WheelProfile(
            name = "Documents",
            countText = "10 / 12",
            extensions = setOf(
                "docx", "docm", "dotx", "dotm", "doc", "dot", "odt", "fodt", "ott", "rtf",
                "wps", "wpd", "lwp", "abw", "zabw", "pages", "sdw", "cwk", "mcw", "602",
                "psw", "pub", "eml", "msg"
            ),
            slices = listOf(
                WheelSliceItem("PDF", null, "pdf"),
                WheelSliceItem("HWPX", null, "hwpx"),
                WheelSliceItem("HWP", null, "hwp"),
                WheelSliceItem("DOCX", null, "docx"),
                WheelSliceItem("ODT", null, "odt"),
                WheelSliceItem("TXT", null, "txt"),
                WheelSliceItem("HTML", null, "html"),
                WheelSliceItem("EPUB", null, "epub"),
                WheelSliceItem("PNG", null, "png"),
                WheelSliceItem("Merge PDF", "PDF", "merge_pdf", isAction = true, actionType = "merge_pdf")
            )
        ),

        // 4. Spreadsheets
        WheelProfile(
            name = "Spreadsheets",
            countText = "9 / 12",
            extensions = setOf(
                "xlsx", "xlsm", "xltx", "xls", "xlt", "ods", "ots", "fods", "csv", "tsv",
                "tab", "et", "numbers", "sdc"
            ),
            slices = listOf(
                WheelSliceItem("PDF", null, "pdf"),
                WheelSliceItem("XLSX", null, "xlsx"),
                WheelSliceItem("CSV", null, "csv"),
                WheelSliceItem("JSON", null, "json"),
                WheelSliceItem("ODS", null, "ods"),
                WheelSliceItem("HWPX", null, "hwpx"),
                WheelSliceItem("HWP", null, "hwp"),
                WheelSliceItem("HTML", null, "html"),
                WheelSliceItem("Merge PDF", "PDF", "merge_pdf", isAction = true, actionType = "merge_pdf")
            )
        ),

        // 5. Presentations
        WheelProfile(
            name = "Presentations",
            countText = "7 / 12",
            extensions = setOf(
                "pptx", "pptm", "potx", "ppsx", "ppt", "pps", "pot", "odp", "otp", "fodp",
                "key", "dps", "sda", "sdd"
            ),
            slices = listOf(
                WheelSliceItem("PDF", null, "pdf"),
                WheelSliceItem("PNG", null, "png"),
                WheelSliceItem("JPG", null, "jpg"),
                WheelSliceItem("PPTX", null, "pptx"),
                WheelSliceItem("HWPX", null, "hwpx"),
                WheelSliceItem("HWP", null, "hwp"),
                WheelSliceItem("Merge PDF", "PDF", "merge_pdf", isAction = true, actionType = "merge_pdf")
            )
        ),

        // 6. HWP / HWPX
        WheelProfile(
            name = "HWP / HWPX",
            countText = "10 / 12",
            extensions = setOf("hwp", "hwpx"),
            slices = listOf(
                WheelSliceItem("PDF", null, "pdf"),
                WheelSliceItem("DOCX", null, "docx"),
                WheelSliceItem("HWPX", null, "hwpx"),
                WheelSliceItem("HWP", null, "hwp"),
                WheelSliceItem("PNG", null, "png"),
                WheelSliceItem("JPG", null, "jpg"),
                WheelSliceItem("TXT", null, "txt"),
                WheelSliceItem("Markdown", null, "md"),
                WheelSliceItem("EPUB", null, "epub"),
                WheelSliceItem("Merge PDF", "PDF", "merge_pdf", isAction = true, actionType = "merge_pdf")
            )
        ),

        // 7. Text & Markdown
        WheelProfile(
            name = "Text & Markdown",
            countText = "7 / 12",
            extensions = setOf(
                "txt", "md", "markdown", "html", "htm", "xhtml", "rst", "tex", "latex",
                "org", "adoc", "asciidoc", "typ", "mediawiki", "wiki", "dbk", "docbook",
                "ipynb", "textile", "man", "opml"
            ),
            slices = listOf(
                WheelSliceItem("PDF", null, "pdf"),
                WheelSliceItem("HWPX", null, "hwpx"),
                WheelSliceItem("HWP", null, "hwp"),
                WheelSliceItem("DOCX", null, "docx"),
                WheelSliceItem("HTML", null, "html"),
                WheelSliceItem("TXT", null, "txt"),
                WheelSliceItem("EPUB", null, "epub")
            )
        ),

        // 8. E-books
        WheelProfile(
            name = "E-books",
            countText = "10 / 12",
            extensions = setOf(
                "epub", "kepub", "mobi", "azw3", "azw", "azw4", "prc", "fb2", "cbz",
                "cbr", "cb7", "cbt", "cbc", "htmlz", "txtz", "chm", "lit", "lrf",
                "pdb", "pml", "rb", "snb", "tcr", "oeb", "opf"
            ),
            slices = listOf(
                WheelSliceItem("PDF", null, "pdf"),
                WheelSliceItem("EPUB", null, "epub"),
                WheelSliceItem("DOCX", null, "docx"),
                WheelSliceItem("TXT", null, "txt"),
                WheelSliceItem("AZW3", null, "azw3"),
                WheelSliceItem("MOBI", null, "mobi"),
                WheelSliceItem("KEPUB", null, "kepub"),
                WheelSliceItem("HWPX", null, "hwpx"),
                WheelSliceItem("HWP", null, "hwp"),
                WheelSliceItem("HTML", null, "html")
            )
        ),

        // 9. Video
        WheelProfile(
            name = "Video",
            countText = "10 / 12",
            extensions = setOf(
                "mp4", "m4v", "mov", "qt", "mkv", "webm", "avi", "divx", "wmv", "asf",
                "flv", "f4v", "mpg", "mpeg", "mpe", "m1v", "m2v", "ts", "m2ts", "mts",
                "m2t", "3gp", "3gpp", "3g2", "ogv", "vob", "mod", "tod", "dv", "dif",
                "mxf", "rm", "rmvb", "swf", "wtv", "dvr-ms", "dvr", "cavs"
            ),
            slices = listOf(
                WheelSliceItem("MP4", null, "mp4"),
                WheelSliceItem("MP4 720p", "MP4", "mp4"),
                WheelSliceItem("MP4 HEVC", "MP4", "mp4"),
                WheelSliceItem("WEBM", null, "webm"),
                WheelSliceItem("MOV", null, "mov"),
                WheelSliceItem("GIF", null, "gif"),
                WheelSliceItem("MP3", null, "mp3"),
                WheelSliceItem("Extract subtitles", "SRT", "srt", isAction = true, actionType = "extract_subtitles"),
                WheelSliceItem("MKV", null, "mkv"),
                WheelSliceItem("AVI", null, "avi")
            )
        ),

        // 10. Audio
        WheelProfile(
            name = "Audio",
            countText = "8 / 12",
            extensions = setOf(
                "mp3", "m4a", "m4b", "aac", "wav", "wave", "flac", "ogg", "oga", "opus",
                "wma", "aiff", "aif", "aifc", "ac3", "amr", "au", "snd", "caf", "weba",
                "mka", "mp2", "mpa", "dss", "voc", "wv", "tta", "w64", "eac3", "ec3",
                "ape", "mpc", "mpp", "mp+", "tak", "dsf", "dff", "dts", "spx"
            ),
            slices = listOf(
                WheelSliceItem("MP3", null, "mp3"),
                WheelSliceItem("MP3 128k", "MP3", "mp3"),
                WheelSliceItem("M4A", null, "m4a"),
                WheelSliceItem("WAV", null, "wav"),
                WheelSliceItem("FLAC", null, "flac"),
                WheelSliceItem("OGG", null, "ogg"),
                WheelSliceItem("OPUS", null, "opus"),
                WheelSliceItem("AAC", null, "aac")
            )
        ),

        // 11. Subtitles
        WheelProfile(
            name = "Subtitles",
            countText = "6 / 12",
            extensions = setOf(
                "srt", "ass", "ssa", "vtt", "smi", "sami", "lrc", "ttml", "dfxp", "sub", "scc"
            ),
            slices = listOf(
                WheelSliceItem("SRT", null, "srt"),
                WheelSliceItem("WebVTT", null, "vtt"),
                WheelSliceItem("ASS", null, "ass"),
                WheelSliceItem("SAMI", null, "smi"),
                WheelSliceItem("LRC", null, "lrc"),
                WheelSliceItem("TTML", null, "ttml")
            )
        ),

        // 12. Vector graphics
        WheelProfile(
            name = "Vector graphics",
            countText = "6 / 12",
            extensions = setOf(
                "svg", "svgz", "eps", "epsf", "epsi", "ps", "ai", "emf", "wmf", "cdr",
                "cgm", "vsd", "vsdx", "odg", "fodg"
            ),
            slices = listOf(
                WheelSliceItem("PNG", null, "png"),
                WheelSliceItem("PDF", null, "pdf"),
                WheelSliceItem("SVG", null, "svg"),
                WheelSliceItem("JPG", null, "jpg"),
                WheelSliceItem("WEBP", null, "webp"),
                WheelSliceItem("EPS", null, "eps")
            )
        ),

        // 13. Archives
        WheelProfile(
            name = "Archives",
            countText = "5 / 12",
            extensions = setOf(
                "zip", "7z", "rar", "tar", "tar.gz", "tgz", "taz", "tar.bz2", "tbz2",
                "tar.bz", "tbz", "tar.xz", "txz", "tar.7z", "tar.lz", "tlz", "tar.z",
                "tz", "gz", "bz2", "bz", "xz", "lz", "lzma", "z", "jar", "cab", "cpio",
                "deb", "rpm", "dmg", "iso", "img", "lha", "lzh", "arj", "alz", "egg"
            ),
            slices = listOf(
                WheelSliceItem("Extract", "Folder", "extract", isAction = true, actionType = "extract"),
                WheelSliceItem("ZIP", null, "zip"),
                WheelSliceItem("7Z", null, "7z"),
                WheelSliceItem("TAR.GZ", null, "tar.gz"),
                WheelSliceItem("TAR", null, "tar")
            )
        ),

        // 14. CAD drawings
        WheelProfile(
            name = "CAD drawings",
            countText = "5 / 12",
            extensions = setOf("dwg", "dxf"),
            slices = listOf(
                WheelSliceItem("PDF", null, "pdf"),
                WheelSliceItem("PNG", null, "png"),
                WheelSliceItem("SVG", null, "svg"),
                WheelSliceItem("DXF", null, "dxf"),
                WheelSliceItem("DWG", null, "dwg")
            )
        ),

        // 15. Fonts
        WheelProfile(
            name = "Fonts",
            countText = "5 / 12",
            extensions = setOf("ttf", "otf", "woff", "woff2", "eot"),
            slices = listOf(
                WheelSliceItem("WOFF2", null, "woff2"),
                WheelSliceItem("WOFF", null, "woff"),
                WheelSliceItem("TTF", null, "ttf"),
                WheelSliceItem("OTF", null, "otf"),
                WheelSliceItem("EOT", null, "eot")
            )
        ),

        // 16. Data
        WheelProfile(
            name = "Data",
            countText = "6 / 12",
            extensions = setOf(
                "json", "jsonl", "ndjson", "yaml", "yml", "toml", "xml", "vcf", "vcard"
            ),
            slices = listOf(
                WheelSliceItem("JSON", null, "json"),
                WheelSliceItem("YAML", null, "yaml"),
                WheelSliceItem("TOML", null, "toml"),
                WheelSliceItem("XML", null, "xml"),
                WheelSliceItem("CSV", null, "csv"),
                WheelSliceItem("XLSX", null, "xlsx")
            )
        ),

        // 17. 3D models
        WheelProfile(
            name = "3D models",
            countText = "6 / 12",
            extensions = setOf("glb", "gltf", "stl", "obj", "3mf", "ply"),
            slices = listOf(
                WheelSliceItem("GLB", null, "glb"),
                WheelSliceItem("STL", null, "stl"),
                WheelSliceItem("OBJ", null, "obj"),
                WheelSliceItem("3MF", null, "3mf"),
                WheelSliceItem("PLY", null, "ply"),
                WheelSliceItem("PNG", null, "png")
            )
        )
    )

    fun getProfileForExtension(rawExt: String): WheelProfile {
        val ext = rawExt.trimStart('.').lowercase()
        return PROFILES.firstOrNull { it.extensions.contains(ext) }
            ?: WheelProfile(
                name = "Archives",
                countText = "5 / 12",
                extensions = emptySet(),
                slices = listOf(
                    WheelSliceItem("Extract", "Folder", "extract", isAction = true, actionType = "extract"),
                    WheelSliceItem("ZIP", null, "zip"),
                    WheelSliceItem("7Z", null, "7z"),
                    WheelSliceItem("TAR.GZ", null, "tar.gz"),
                    WheelSliceItem("TAR", null, "tar")
                )
            )
    }
}
