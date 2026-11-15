// WebAudio-вывод для kira.
//
// Kira в wasm только декодирует OGG и один раз отдаёт PCM в этот скрипт.
// Дальше играет штатный WebAudio-граф браузера: AudioBufferSourceNode для
// one-shot звуков, GainNode для громкости, playbackRate для замедления.
//
// Скрипт должен подключаться после mq_js_bundle.js: он использует глобальные
// wasm_memory и miniquad_add_plugin.
(function () {
    "use strict";

    var context = null;
    var master = null;
    var buffers = [];
    var activeSources = [];
    var loopSource = null;
    var loopGain = null;
    var ready = false;

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
            context = new AudioContextClass({ latencyHint: "interactive" });
        } catch (error) {
            try {
                context = new AudioContextClass();
            } catch (fallbackError) {
                console.error("DS audio: не удалось создать AudioContext", fallbackError);
                context = null;
                return null;
            }
        }

        try {
            master = context.createGain();
            master.gain.value = 0.5;
            master.connect(context.destination);
        } catch (error) {
            console.error("DS audio: не удалось создать master-узел", error);
            try {
                context.close();
            } catch (closeError) {
                console.warn("DS audio: не удалось закрыть AudioContext", closeError);
            }
            context = null;
            master = null;
            return null;
        }

        buffers = [];
        activeSources = [];
        ready = true;
        return context;
    }

    function audioUpload(id, samples, frames, channels, sampleRate) {
        if (!ready || context === null) {
            return 0;
        }

        if (typeof wasm_memory === "undefined" || wasm_memory === null) {
            return 0;
        }

        if (frames === 0 || channels === 0) {
            return 0;
        }

        try {
            var interleaved = new Float32Array(wasm_memory.buffer, samples, frames * channels);
            var buffer = context.createBuffer(channels, frames, sampleRate);

            for (var channel = 0; channel < channels; channel++) {
                var output = buffer.getChannelData(channel);
                for (var frame = 0; frame < frames; frame++) {
                    output[frame] = interleaved[frame * channels + channel];
                }
            }

            buffers[id] = buffer;
            return 1;
        } catch (error) {
            console.error("DS audio: не удалось загрузить звук", error);
            return 0;
        }
    }

    function audioPlay(id, rate, gain) {
        if (!ready || context === null || buffers[id] === undefined || gain <= 0.0) {
            return;
        }

        try {
            var source = context.createBufferSource();
            source.buffer = buffers[id];
            source.playbackRate.value = rate;

            var volume = context.createGain();
            volume.gain.value = gain;

            source.connect(volume);
            volume.connect(master);

            var entry = { source: source, volume: volume };
            activeSources.push(entry);

            source.onended = function () {
                var index = activeSources.indexOf(entry);
                if (index !== -1) {
                    activeSources.splice(index, 1);
                }
                try {
                    source.disconnect();
                    volume.disconnect();
                } catch (error) {
                    // Узел уже мог быть отключён при shutdown.
                }
            };

            source.start();
        } catch (error) {
            console.error("DS audio: не удалось проиграть звук", error);
        }
    }

    function audioLoopStart(id, gain) {
        audioLoopStop();

        if (!ready || context === null || buffers[id] === undefined) {
            return;
        }

        try {
            loopSource = context.createBufferSource();
            loopSource.buffer = buffers[id];
            loopSource.loop = true;

            loopGain = context.createGain();
            loopGain.gain.value = gain;

            loopSource.connect(loopGain);
            loopGain.connect(master);
            loopSource.start();
        } catch (error) {
            console.error("DS audio: не удалось запустить луп", error);
            audioLoopStop();
        }
    }

    function audioLoopGain(gain) {
        if (loopGain === null || context === null) {
            return;
        }

        try {
            loopGain.gain.setTargetAtTime(gain, context.currentTime, 0.01);
        } catch (error) {
            loopGain.gain.value = gain;
        }
    }

    function audioLoopStop() {
        if (loopSource !== null) {
            try {
                loopSource.stop();
            } catch (error) {
                // Источник мог ещё не успеть стартовать.
            }
            try {
                loopSource.disconnect();
            } catch (error) {
                // Узел уже отключён.
            }
            loopSource = null;
        }

        if (loopGain !== null) {
            try {
                loopGain.disconnect();
            } catch (error) {
                // Узел уже отключён.
            }
            loopGain = null;
        }
    }

    function audioSetMaster(gain) {
        if (master === null || context === null) {
            return;
        }

        try {
            master.gain.setTargetAtTime(gain, context.currentTime, 0.02);
        } catch (error) {
            master.gain.value = gain;
        }
    }

    function audioShutdown() {
        audioLoopStop();

        for (var i = 0; i < activeSources.length; i++) {
            var entry = activeSources[i];
            try {
                entry.source.stop();
            } catch (error) {
                // Источник уже остановлен.
            }
            try {
                entry.source.disconnect();
                entry.volume.disconnect();
            } catch (error) {
                // Узлы уже отключены.
            }
        }
        activeSources = [];

        if (master !== null) {
            try {
                master.disconnect();
            } catch (error) {
                // Узел уже отключён.
            }
            master = null;
        }

        if (context !== null) {
            try {
                context.close().catch(function () {});
            } catch (error) {
                // Контекст уже закрыт.
            }
            context = null;
        }

        buffers = [];
        ready = false;
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

    function registerPlugin(imports) {
        imports.env.ds_audio_init = audioInit;
        imports.env.ds_audio_start = resumeContext;
        imports.env.ds_audio_upload = audioUpload;
        imports.env.ds_audio_play = audioPlay;
        imports.env.ds_audio_loop_start = audioLoopStart;
        imports.env.ds_audio_loop_gain = audioLoopGain;
        imports.env.ds_audio_loop_stop = audioLoopStop;
        imports.env.ds_audio_set_master = audioSetMaster;
        imports.env.ds_audio_shutdown = audioShutdown;
    }

    miniquad_add_plugin({
        register_plugin: registerPlugin,
        on_init: onInit,
        name: "ds_audio",
        version: 2,
    });
})();
