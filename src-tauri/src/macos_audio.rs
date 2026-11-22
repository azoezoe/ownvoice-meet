// A global CoreAudio process tap: system playback even when a headset exposes both input and
// output. CPAL's output-device fallback would open the headset microphone in that case.
// The private aggregate is visible only to this process and is destroyed after its stream.
use std::ffi::CStr;
use std::ptr::NonNull;
use std::sync::Mutex;

use objc2::{runtime::AnyObject, AnyThread};
use objc2_core_audio::*;
use objc2_core_audio_types::{AudioBuffer, AudioBufferList, AudioStreamBasicDescription, AudioTimeStamp, kAudioFormatFlagIsFloat};
use objc2_core_foundation::CFDictionary;
use objc2_foundation::{NSArray, NSDictionary, NSNumber, NSString};

pub struct SystemAudio {
    tap: AudioObjectID,
    aggregate: AudioObjectID,
    io_proc: AudioDeviceIOProcID,
    callback: Box<Callback>,
}

struct Callback {
    rate: u32,
    sink: Mutex<Box<dyn FnMut(Vec<f32>, u32) + Send>>,
}

/// CoreAudio owns the buffer memory for this callback only. The boxed client data remains
/// stable until Stop + DestroyIOProcID have returned, before SystemAudio drops its callback.
unsafe extern "C-unwind" fn input(
    _device: AudioObjectID, _now: NonNull<AudioTimeStamp>, data: NonNull<AudioBufferList>,
    _input_time: NonNull<AudioTimeStamp>, _out: NonNull<AudioBufferList>,
    _output_time: NonNull<AudioTimeStamp>, client: *mut std::ffi::c_void,
) -> i32 {
    let list = data.as_ref();
    let buffers = std::slice::from_raw_parts(list.mBuffers.as_ptr(), list.mNumberBuffers as usize);
    let mono = mix_buffers(buffers);
    if mono.is_empty() { return 0; }
    let callback = &*(client.cast::<Callback>());
    if let Ok(mut sink) = callback.sink.lock() { sink(mono, callback.rate); }
    0
}

// Each non-null buffer must point to mDataByteSize bytes of native Float32 PCM, as validated
// through the tap's ASBD. Support both interleaved and planar buffers without reading past either.
unsafe fn mix_buffers(buffers: &[AudioBuffer]) -> Vec<f32> {
    let frames = buffers.iter().filter(|b| b.mNumberChannels > 0 && !b.mData.is_null())
        .map(|b| b.mDataByteSize as usize / 4 / b.mNumberChannels as usize).min().unwrap_or(0);
    if frames == 0 { return Vec::new(); }
    let mut mono = vec![0.0; frames];
    let mut channels = 0;
    for b in buffers.iter().filter(|b| b.mNumberChannels > 0 && !b.mData.is_null()) {
        let count = b.mNumberChannels as usize;
        let samples = std::slice::from_raw_parts(b.mData.cast::<f32>(), frames * count);
        for (dst, frame) in mono.iter_mut().zip(samples.chunks_exact(count)) {
            *dst += frame.iter().sum::<f32>();
        }
        channels += count;
    }
    for x in &mut mono { *x /= channels as f32; }
    mono
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buffer(data: &mut [f32], channels: u32) -> AudioBuffer {
        AudioBuffer { mNumberChannels: channels, mDataByteSize: std::mem::size_of_val(data) as u32, mData: data.as_mut_ptr().cast() }
    }

    #[test]
    fn stereo_interleaved_and_planar_produce_the_same_mono() {
        let mut interleaved = [0.8, 0.2, -0.4, 0.2];
        let mut left = [0.8, -0.4];
        let mut right = [0.2, 0.2];
        let a = unsafe { mix_buffers(&[buffer(&mut interleaved, 2)]) };
        let b = unsafe { mix_buffers(&[buffer(&mut left, 1), buffer(&mut right, 1)]) };
        assert_eq!(a, b);
        assert_eq!(a, vec![0.5, -0.1]);
    }

    #[test]
    fn disabled_and_short_buffers_are_safe() {
        let disabled = AudioBuffer { mNumberChannels: 2, mDataByteSize: 8192, mData: std::ptr::null_mut() };
        assert!(unsafe { mix_buffers(&[disabled]) }.is_empty());
        let mut left = [0.8, 0.4];
        let mut right = [0.2];
        assert_eq!(unsafe { mix_buffers(&[buffer(&mut left, 1), buffer(&mut right, 1), disabled]) }, vec![0.5]);
    }
}

fn key(value: &CStr) -> objc2::rc::Retained<NSString> {
    NSString::from_str(value.to_str().expect("CoreAudio dictionary key is ASCII"))
}

fn check(status: i32, step: &str) -> Result<(), String> {
    if status == 0 { Ok(()) } else {
        Err(format!("{step}（CoreAudio {status}）；請到系統設定 → 隱私權與安全性 → 螢幕與系統音訊錄製，允許 Zoe Meet，然後重新開啟程式"))
    }
}

impl SystemAudio {
    pub fn open(on_audio: impl FnMut(Vec<f32>, u32) + Send + 'static) -> Result<Self, String> {
        // SAFETY: all Objective-C objects remain retained through each FFI call. CoreAudio
        // outputs are initialized to zero and checked before use. macOS minimum is 14.6.
        unsafe {
            let desc = CATapDescription::initStereoGlobalTapButExcludeProcesses(
                CATapDescription::alloc(), &NSArray::<NSNumber>::new(),
            );
            desc.setPrivate(true);
            desc.setMuteBehavior(CATapMuteBehavior::Unmuted);
            desc.setName(&NSString::from_str("Zoe Meet system sound"));
            let mut capture = Self {
                tap: 0, aggregate: 0, io_proc: None,
                callback: Box::new(Callback { rate: 0, sink: Mutex::new(Box::new(on_audio)) }),
            };
            check(AudioHardwareCreateProcessTap(Some(&desc), &mut capture.tap), "無法建立系統收音")?;

            let address = AudioObjectPropertyAddress {
                mSelector: kAudioTapPropertyFormat, mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain,
            };
            let mut format: AudioStreamBasicDescription = std::mem::zeroed();
            let mut size = std::mem::size_of_val(&format) as u32;
            check(AudioObjectGetPropertyData(capture.tap, NonNull::from(&address), 0,
                std::ptr::null(), NonNull::from(&mut size), NonNull::from(&mut format).cast()), "無法讀取系統收音格式")?;
            if format.mFormatFlags & kAudioFormatFlagIsFloat == 0 || format.mBitsPerChannel != 32 || format.mSampleRate <= 0.0 {
                return Err("系統收音格式不支援（需要 32-bit float PCM）".into());
            }
            capture.callback.rate = format.mSampleRate as u32;

            let tap_uid = desc.UUID().UUIDString();
            let uid = format!("io.github.azoezoe.zoe-meet.audio.{}", tap_uid);
            let uid_string = NSString::from_str(&uid);
            let name = NSString::from_str("Zoe Meet system sound");
            let yes = NSNumber::new_bool(true);
            let tap_keys = [key(kAudioSubTapUIDKey), key(kAudioSubTapDriftCompensationKey)];
            let tap = NSDictionary::<NSString, AnyObject>::from_slices(
                &[&*tap_keys[0], &*tap_keys[1]], &[&*tap_uid, &*yes],
            );
            let taps = NSArray::from_retained_slice(&[tap]);
            let keys = [
                key(kAudioAggregateDeviceNameKey), key(kAudioAggregateDeviceUIDKey),
                key(kAudioAggregateDeviceTapListKey), key(kAudioAggregateDeviceTapAutoStartKey),
                key(kAudioAggregateDeviceIsPrivateKey),
            ];
            let config = NSDictionary::<NSString, AnyObject>::from_slices(
                &[&*keys[0], &*keys[1], &*keys[2], &*keys[3], &*keys[4]],
                &[&*name, &*uid_string, &*taps, &*yes, &*yes],
            );
            // NSDictionary and CFDictionary are toll-free bridged. Neither is mutated here.
            let cf_config = &*((&*config as *const NSDictionary<NSString, AnyObject>).cast::<CFDictionary>());
            check(AudioHardwareCreateAggregateDevice(cf_config, NonNull::from(&mut capture.aggregate)), "無法開啟系統收音裝置")?;
            check(AudioDeviceCreateIOProcID(capture.aggregate, Some(input),
                (&mut *capture.callback as *mut Callback).cast(), NonNull::from(&mut capture.io_proc)), "無法連接系統收音")?;
            check(AudioDeviceStart(capture.aggregate, capture.io_proc), "無法開始系統收音")?;
            Ok(capture)
        }
    }
}

impl Drop for SystemAudio {
    fn drop(&mut self) {
        // The audio stream must have stopped before dropping this owner.
        unsafe {
            if self.io_proc.is_some() {
                let _ = AudioDeviceStop(self.aggregate, self.io_proc);
                let _ = AudioDeviceDestroyIOProcID(self.aggregate, self.io_proc);
            }
            if self.aggregate != 0 { let _ = AudioHardwareDestroyAggregateDevice(self.aggregate); }
            if self.tap != 0 { let _ = AudioHardwareDestroyProcessTap(self.tap); }
        }
    }
}
