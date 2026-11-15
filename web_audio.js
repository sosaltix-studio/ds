// WebAudio-бэкенд для kira.
//
// Штатный wasm-бэкенд kira использует wasm-bindgen и не работает с загрузчиком
// miniquad, поэтому звук тянется из wasm через ScriptProcessorNode:
// браузер вызывает ds_audio_render(), а Rust рендерит очередной блок сэмплов.
//
// Скрипт должен подключаться после mq_js_bundle.js: он использует глобальные
// wasm_exports, wasm_memory и miniquad_add_plugin.
(function () {
    "use strict";

    var CHANNELS = 2;
    // Не должно превышать MAX_FRAMES в src/wasm_audio.rs.
    var BUFFER_FRAMES = 1024;

    var context = null;
    var processor = null;
    var silentSource = null;
    var started = false;

    function ensureContext() {
        if (context !== null) {
            return context;
        }

        var AudioContextClass = window.AudioContext || window.webkitAudioContext;
        if (AudioContextClass === undefined) {
            console.error("DS audio: WebAudio не поддерживается браузером");
            return null;
        }

        try {
            context = new AudioContextClass({ sampleRate: 44100, latencyHint: "interactive" });
        } catch (error) {
            try {
                context = new AudioContextClass();
            } catch (fallbackError) {
                console.error("DS audio: не удалось создать AudioContext", fallbackError);
                context = null;
                return null;
            }
        }

        processor = null;

        try {
            processor = context.createScriptProcessor(BUFFER_FRAMES, 1, CHANNELS);

            // Тихий источник нужен, чтобы граф гарантированно тянул ScriptProcessor
            // даже когда к нему не подключено ничего другого.
            var silence = context.createBuffer(1, 1, context.sampleRate);
            silentSource = context.createBufferSource();
            silentSource.buffer = silence;
            silentSource.loop = true;
            silentSource.connect(processor);
            try {
                silentSource.start();
            } catch (error) {
                console.warn("DS audio: не удалось запустить тихий источник", error);
            }

            processor.onaudioprocess = renderAudio;
        } catch (error) {
            console.error("DS audio: не удалось создать ScriptProcessor", error);
            try {
                context.close();
            } catch (closeError) {
                console.warn("DS audio: не удалось закрыть AudioContext", closeError);
            }
            silentSource = null;
            processor = null;
            context = null;
            return null;
        }

        return context;
    }

    function renderAudio(event) {
        var frames = event.outputBuffer.length;
        var left = event.outputBuffer.getChannelData(0);
        var right = event.outputBuffer.getChannelData(1);

        if (typeof wasm_exports === "undefined" || wasm_exports === null || !wasm_memory) {
            left.fill(0);
            right.fill(0);
            return;
        }

        if (typeof wasm_exports.ds_audio_render !== "function") {
            left.fill(0);
            right.fill(0);
            return;
        }

        try {
            wasm_exports.ds_audio_render(frames);

            var pointer = wasm_exports.ds_audio_buffer_ptr();
            var samples = new Float32Array(wasm_memory.buffer, pointer, frames * CHANNELS);

            for (var i = 0; i < frames; i++) {
                left[i] = samples[i * CHANNELS];
                right[i] = samples[i * CHANNELS + 1];
            }
        } catch (error) {
            left.fill(0);
            right.fill(0);
            console.error("DS audio: ошибка рендеринга", error);
        }
    }

    function resumeContext() {
        if (context !== null && context.state !== "running") {
            context.resume().catch(function () {});
        }
    }

    function onInit() {
        // Браузеры запускают AudioContext только после действия пользователя.
        ["pointerdown", "mousedown", "keydown", "touchstart"].forEach(function (name) {
            window.addEventListener(name, resumeContext, { passive: true });
        });
    }

    function audioInit() {
        var ctx = ensureContext();
        return ctx === null ? 0 : ctx.sampleRate;
    }

    function audioStart() {
        if (context === null) {
            return;
        }

        if (!started) {
            processor.connect(context.destination);
            started = true;
        }

        resumeContext();
    }

    function registerPlugin(imports) {
        imports.env.ds_audio_init = audioInit;
        imports.env.ds_audio_start = audioStart;
    }

    miniquad_add_plugin({
        register_plugin: registerPlugin,
        on_init: onInit,
        name: "ds_audio",
        version: 1,
    });
})();
