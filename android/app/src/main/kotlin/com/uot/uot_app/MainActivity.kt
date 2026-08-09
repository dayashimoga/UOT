package com.uot.uot_app

import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.EventChannel
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {
    private var blePlugin: BleAdapterPlugin? = null
    private var wifiDirectPlugin: WifiDirectAdapterPlugin? = null

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        // Register BLE Adapter Plugin — guarded per platform feature
        try {
            if (context.packageManager.hasSystemFeature("android.hardware.bluetooth_le")) {
                val ble = BleAdapterPlugin(context)
                blePlugin = ble
                MethodChannel(flutterEngine.dartExecutor.binaryMessenger, BleAdapterPlugin.CHANNEL_NAME)
                    .setMethodCallHandler(ble)
                EventChannel(flutterEngine.dartExecutor.binaryMessenger, BleAdapterPlugin.STATE_CHANNEL)
                    .setStreamHandler(ble.createStateStreamHandler())
                EventChannel(flutterEngine.dartExecutor.binaryMessenger, BleAdapterPlugin.SCAN_CHANNEL)
                    .setStreamHandler(ble.createScanStreamHandler())
            } else {
                android.util.Log.w("UOT_MAIN", "BLE not available on this device, skipping BleAdapterPlugin")
            }
        } catch (e: Exception) {
            android.util.Log.e("UOT_MAIN", "BleAdapterPlugin registration failed: ${e.message}", e)
        }

        // Register Wi-Fi Direct Plugin — guarded per platform feature
        try {
            if (context.packageManager.hasSystemFeature("android.hardware.wifi.direct")) {
                val wifi = WifiDirectAdapterPlugin(context)
                wifiDirectPlugin = wifi
                MethodChannel(flutterEngine.dartExecutor.binaryMessenger, WifiDirectAdapterPlugin.CHANNEL_NAME)
                    .setMethodCallHandler(wifi)
                EventChannel(flutterEngine.dartExecutor.binaryMessenger, WifiDirectAdapterPlugin.STATE_CHANNEL)
                    .setStreamHandler(wifi.createStateStreamHandler())
                EventChannel(flutterEngine.dartExecutor.binaryMessenger, WifiDirectAdapterPlugin.PEER_CHANNEL)
                    .setStreamHandler(wifi.createPeerStreamHandler())
            } else {
                android.util.Log.w("UOT_MAIN", "Wi-Fi Direct not available on this device, skipping WifiDirectAdapterPlugin")
            }
        } catch (e: Exception) {
            android.util.Log.e("UOT_MAIN", "WifiDirectAdapterPlugin registration failed: ${e.message}", e)
        }
    }

    override fun onDestroy() {
        blePlugin?.dispose()
        wifiDirectPlugin?.dispose()
        super.onDestroy()
    }
}
