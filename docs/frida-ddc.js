// Observe Windows monitor API calls; do not alter arguments or return values.
'use strict';

const attached = new Set();
let sequence = 0;
let stepSequence = 0;
let activeStep = null;
const session = new Date().toISOString() + ':' + Process.id;
function log(event) {
    console.log(JSON.stringify({time: new Date().toISOString(), session, ...event}));
}
function hex(value) { return '0x' + value.toString(16).padStart(2, '0'); }

// These helpers are called in the Frida REPL. They only annotate the log.
globalThis.mark = function (label, context = '') {
    if (typeof label !== 'string' || label.trim() === '') {
        throw new Error('Podaj etykiete, np. mark("Full Game -> Wide")');
    }
    if (activeStep !== null) {
        throw new Error('Najpierw zakoncz poprzedni krok: done("obserwacja")');
    }
    activeStep = {stepId: ++stepSequence, label: label.trim(), context: String(context)};
    log({event: 'step-start', ...activeStep});
    return 'Teraz kliknij tylko wskazana opcje. Po odpowiedziach wpisz done("efekt w OSD/obrazie").';
};
globalThis.done = function (observation) {
    if (activeStep === null) throw new Error('Brak aktywnego kroku; najpierw mark("etykieta").');
    if (typeof observation !== 'string' || observation.trim() === '') {
        throw new Error('Opisz efekt albo wpisz done("nie sprawdzono efektu").');
    }
    log({event: 'step-end', ...activeStep, observation: observation.trim()});
    activeStep = null;
    return 'Krok zapisany.';
};
globalThis.note = function (text) {
    log({event: 'note', stepId: activeStep === null ? null : activeStep.stepId,
        text: String(text)});
};

function hook(module, name) {
    const address = module.findExportByName(name);
    if (address === null || attached.has(address.toString())) return;
    try {
        Interceptor.attach(address, {
            onEnter(args) {
                this.startedMs = Date.now();
                this.id = ++sequence;
                this.monitor = args[0].toString();
                // Keep the step from call entry even if the user closes it
                // while an asynchronous/native call is still in progress.
                this.step = activeStep === null ? {stepId: null, label: null} : {...activeStep};
                const event = {event: 'call', id: this.id, function: name,
                    monitor: this.monitor, thread: this.threadId, ...this.step};
                if (name !== 'SaveCurrentMonitorSettings') {
                    event.code = hex(args[1].toUInt32() & 255);
                }
                if (name === 'SetVCPFeature') {
                    event.valueDecimal = args[2].toUInt32();
                    event.valueHex = hex(event.valueDecimal);
                }
                if (name === 'GetVCPFeatureAndVCPFeatureReply') {
                    this.type = args[2];
                    this.current = args[3];
                    this.maximum = args[4];
                }
                log(event);
            },
            onLeave(retval) {
                const lastError = this.lastError;
                const ok = retval.toInt32() !== 0;
                const event = {event: 'return', id: this.id, function: name,
                    ok, lastError, ...this.step, durationMs: Date.now() - this.startedMs};
                if (!ok && typeof lastError === 'number') {
                    event.lastErrorHex = hex(lastError >>> 0);
                }
                if (ok && name === 'GetVCPFeatureAndVCPFeatureReply') {
                    try {
                        if (!this.type.isNull()) event.type = this.type.readU32();
                        if (!this.current.isNull()) event.current = this.current.readU32();
                        if (!this.maximum.isNull()) event.maximum = this.maximum.readU32();
                    } catch (error) { event.readError = String(error); }
                }
                log(event);
            }
        });
        attached.add(address.toString());
        log({event: 'hook', module: module.name, function: name, address: address.toString()});
    } catch (error) {
        log({event: 'hook-error', module: module.name, function: name, error: String(error)});
    }
}

log({event: 'start', scriptVersion: 2, platform: Process.platform, arch: Process.arch, pid: Process.id});
// Includes libraries already present and ones loaded after attaching.
const observer = Process.attachModuleObserver({
    onAdded(module) {
        log({event: 'module', name: module.name, path: module.path});
        for (const name of ['SetVCPFeature', 'GetVCPFeatureAndVCPFeatureReply',
            'SaveCurrentMonitorSettings']) hook(module, name);
    }
});
