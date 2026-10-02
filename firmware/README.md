# Titan Army P275MV PLUS firmware

Official firmware packages from Titan Army (泰坦军团). The files are renamed and the Chinese notes are translated
below. The binaries are unmodified (SHA-256 below).

> [!WARNING]
> ## ⚠️ Use at your own risk
>
> Flashing the wrong file or interrupting an update can leave the monitor unable to start. **You do all of this at
> your own risk, and the author is not 100 % sure it is right.** The tables, the translations and the way of telling
> the two monitor versions apart (see the
> [FAQ](../FAQ.md#how-do-i-tell-the-p275mv-plus-from-the-p275mv-plus-enhanced)) come from the author's own
> observations and an amateur translation of Chinese documents, and may contain mistakes. **The authors give no
> warranty (see the [MIT license](../LICENSE)) and are not responsible for any damage.** If in doubt, ask the
> manufacturer's support before flashing.
>
> Check your model first, keep the monitor powered during the whole update and follow every step. Titan Control does
> not flash firmware; this is done from the monitor's own OSD with a USB stick.

> [!NOTE]
> ## 🤖 Developed with an LLM
>
> This page was written and translated with the help of a large language model (LLM, Claude) and checked by the
> author, but **it may still contain mistakes. Compare it with the original guide (`usb-update-guide.zh.pdf`) before
> you flash.**

## Which file for which monitor

| File                                           | Monitor                                | Firmware                   | Original package                                                                                                                                                                              |
| ---------------------------------------------- | -------------------------------------- | -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `P275MV_PLUS_M001_20241211_5.1.1.bin`          | P275MV PLUS (regular)                  | M001 2024-12-11, USB 5.1.1 | _P275MV PLUS Firmware Update (M001 20241211 5.1.1)_, file `3、P275MV PLUS - M001-20241211-5.1.1.bin`                                                                                          |
| `P275MV_PLUS_Enhanced_M001_20250920_5.1.1.bin` | **P275MV PLUS Enhanced (增强版) only** | M001 2025-09-20, USB 5.1.1 | _P275MV PLUS（增强版）固件补丁升级包（M001 20250920 5.1.1）_, file `105.001.38151_TITAN_2751V_PLUS_TKJT2023_W9RQUHD_QA2_2H1DP1C_M270QAN07_6_UHD160_0x98_M001_20250920_XGN3.0.4_BCE8_售后.bin` |
| `usb-update-guide.zh.pdf`                      | —                                      | —                          | The original update guide with pictures (Chinese), translated in [USB update](#usb-update)                                                                                                    |

The Enhanced package is a service (售后, after-sales) build and is no longer on the manufacturer's site. It comes with
three warning files that all say:

> This firmware can only be used on the Titan Army P275MV PLUS (Enhanced) monitor! It must not be used on the
> P275MV, the P275MV PLUS or any other model!

The Enhanced build name describes the hardware: board `TKJT2023`, panel `M270QAN07.6` (27" UHD 160 Hz Mini LED),
2× HDMI, 1× DP, 1× USB-C.

```text
7f94a52fea0275216f42283cedaacdbf1a0996aea26d146ae9869d0978b26373  P275MV_PLUS_Enhanced_M001_20250920_5.1.1.bin
c8beda8f4cc03b7de1a394350744605b9baf04e9f201ced1c480b0cb3c7d3ad3  P275MV_PLUS_M001_20241211_5.1.1.bin
```

## Version history of the Enhanced firmware

Translated from `1、请先阅读本文档.txt` ("Read this first"):

- **M001 2025-04-24 5.1.1**: first release.
- **M001 2025-07-24 5.1.1**
  - The HDR Movie color temperature is tuned with the SOP 2.0 process. This fixes the too-low color temperature of
    SDR colors in HDR Movie mode.
  - SDR color calibration is tuned with SOP 2.0.
- **M001 2025-09-20 5.1.1**
  - Mini LED backlight firmware updated to LDFW4.
  - **VRR added to DyDs: Adaptive-Sync can now stay on while a DyDs picture mode is active.**
  - HDR Movie color temperature and SDR color calibration tuned further with SOP 3.1.

Notes from the same file:

1. A firmware has an architecture version, a software version and a USB software version. The OSD shows only the
   USB software version (5.1.1). It changes only when the USB management software is updated, so it stays 5.1.1 after
   these updates. The real software version is the date stamp (for example 20250920).
2. The OSD cannot show the firmware date yet. Over DDC/CI the monitor also reports only 5.1.1 (`VCP FE = 0000/5101`),
   which is why Titan Control asks you which package is installed (_Monitor → Information_).
3. When to update: when a feature from the list above is missing in your OSD.
4. In the update guide the text is binding; the pictures are only illustrations.
5. Read the whole guide before you start.
6. If the update is not carried through to the end, the monitor shows various bugs. This is expected: finish the
   whole procedure and the update completes normally.

## USB update

> [!IMPORTANT]
> Keep the monitor powered during the whole update. Never switch it off or unplug it while the firmware is being
> written: the monitor would not start again.

### Preparing the USB stick

1. The monitor only reads **FAT32** sticks with **a single partition of at most 4000 MB**. A larger partition is not
   accepted and the update will not start; on a bigger stick create a 4000 MB partition and leave the rest unused.
   Sticks with partitions that cannot be deleted may not work.
2. Once copied to the stick, the firmware file must be renamed to **`MERGE.bin`**.
3. To prepare the stick on Linux (replace `sdX` with your stick; this erases it):

   ```bash
   lsblk                                   # find the stick
   sudo wipefs -a /dev/sdX
   echo 'type=0c, size=4000M' | sudo sfdisk /dev/sdX
   sudo mkfs.vfat -F 32 /dev/sdX1
   ```

   On Windows (from the original guide): open _Disk Management_, delete every volume on the stick, then _New Simple
   Volume_ with size 4000 MB and file system FAT32, other options default.

### Updating

1. Connect the monitor to the computer and make sure it shows a picture.
2. Insert the stick into the computer. It must have exactly 1 partition, larger than 1 GB and at most 4000 MB,
   formatted as FAT32.
3. Copy the firmware file (`*.bin`) to the stick and rename it to `MERGE.bin`.
4. Eject the stick and plug it into the monitor's **USB Type-A port closest to the USB Type-B upstream port**.
5. Turn HDR off in the operating system. In the monitor OSD, run _System Settings → Factory Reset_.
6. In _System Settings → Software Update_, choose _USB Update_ and confirm with _Yes_.
7. The monitor disconnects its USB devices during the update: confirm.
8. The monitor asks you not to cut the power. Leave it alone.
9. The button indicator blinks white during the update.
10. The update takes about 30 seconds. When it reports success, remove the stick and wait for the monitor to restart
    by itself. Do not switch it off or unplug it. A red light next to the white indicator during the restart is
    normal; the restart usually takes about 10 seconds.
11. After the restart the screen may go black 1–3 times. Wait about 30 seconds, then, **without switching the monitor
    off**, pull its power cable and plug it back in. When the monitor has started, run _System Settings → Factory
    Reset_ in the OSD again. After the reset the update is complete.

After updating to 2025-09-20, select **2025-09-20** under _Monitor → Information → Installed firmware package_ in
Titan Control, so it allows DyDs together with Adaptive-Sync.
