import Foundation
import CoreBluetooth
import Flutter

/// Native iOS BLE GATT Adapter using CoreBluetooth
///
/// Implements CBPeripheralManager for advertising and CBCentralManager for scanning.
class BleAdapterPlugin: NSObject, FlutterPlugin, FlutterStreamHandler {
    
    static let channelName = "com.uot.ble/adapter"
    static let stateChannelName = "com.uot.ble/state_stream"
    static let scanChannelName = "com.uot.ble/scan_stream"
    
    private var peripheralManager: CBPeripheralManager?
    private var centralManager: CBCentralManager?
    private var stateEventSink: FlutterEventSink?
    private var scanEventSink: FlutterEventSink?
    private var serviceUUID: CBUUID?
    private var controlCharUUID: CBUUID?
    private var dataCharUUID: CBUUID?
    
    static func register(with registrar: FlutterPluginRegistrar) {
        let channel = FlutterMethodChannel(name: channelName, binaryMessenger: registrar.messenger())
        let instance = BleAdapterPlugin()
        registrar.addMethodCallDelegate(instance, channel: channel)
        
        let stateChannel = FlutterEventChannel(name: stateChannelName, binaryMessenger: registrar.messenger())
        stateChannel.setStreamHandler(instance)
        
        let scanChannel = FlutterEventChannel(name: scanChannelName, binaryMessenger: registrar.messenger())
        scanChannel.setStreamHandler(BleAdapterScanStreamHandler.shared)
    }
    
    func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
        switch call.method {
        case "initialize":
            handleInitialize(call, result: result)
        case "startAdvertising":
            handleStartAdvertising(call, result: result)
        case "stopAdvertising":
            handleStopAdvertising(result: result)
        case "startScanning":
            handleStartScanning(call, result: result)
        case "stopScanning":
            handleStopScanning(result: result)
        case "sendData":
            result(false) // TODO: implement
        default:
            result(FlutterMethodNotImplemented)
        }
    }
    
    private func handleInitialize(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
        guard let args = call.arguments as? [String: Any] else {
            result(["supported": false])
            return
        }
        
        serviceUUID = CBUUID(string: args["serviceUuid"] as? String ?? "")
        controlCharUUID = CBUUID(string: args["charControlUuid"] as? String ?? "")
        dataCharUUID = CBUUID(string: args["charDataUuid"] as? String ?? "")
        
        peripheralManager = CBPeripheralManager(delegate: self, queue: nil)
        centralManager = CBCentralManager(delegate: self, queue: nil)
        
        result(["supported": true])
    }
    
    private func handleStartAdvertising(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
        guard let pm = peripheralManager, let uuid = serviceUUID else {
            result(false)
            return
        }
        
        pm.startAdvertising([
            CBAdvertisementDataServiceUUIDsKey: [uuid],
            CBAdvertisementDataLocalNameKey: "UOT-Device"
        ])
        result(true)
    }
    
    private func handleStopAdvertising(result: @escaping FlutterResult) {
        peripheralManager?.stopAdvertising()
        stateEventSink?("poweredOn")
        result(true)
    }
    
    private func handleStartScanning(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
        guard let cm = centralManager, let uuid = serviceUUID else {
            result(false)
            return
        }
        cm.scanForPeripherals(withServices: [uuid], options: [
            CBCentralManagerScanOptionAllowDuplicatesKey: false
        ])
        stateEventSink?("scanning")
        result(true)
    }
    
    private func handleStopScanning(result: @escaping FlutterResult) {
        centralManager?.stopScan()
        stateEventSink?("poweredOn")
        result(true)
    }
    
    // FlutterStreamHandler
    func onListen(withArguments arguments: Any?, eventSink events: @escaping FlutterEventSink) -> FlutterError? {
        stateEventSink = events
        return nil
    }
    
    func onCancel(withArguments arguments: Any?) -> FlutterError? {
        stateEventSink = nil
        return nil
    }
}

extension BleAdapterPlugin: CBPeripheralManagerDelegate {
    func peripheralManagerDidUpdateState(_ peripheral: CBPeripheralManager) {
        switch peripheral.state {
        case .poweredOn:
            // Add GATT service
            if let uuid = serviceUUID, let ctrlUUID = controlCharUUID, let dataUUID = dataCharUUID {
                let service = CBMutableService(type: uuid, primary: true)
                let controlChar = CBMutableCharacteristic(
                    type: ctrlUUID,
                    properties: [.write, .notify],
                    value: nil,
                    permissions: [.writeable, .readable]
                )
                let dataChar = CBMutableCharacteristic(
                    type: dataUUID,
                    properties: [.writeWithoutResponse, .notify],
                    value: nil,
                    permissions: [.writeable, .readable]
                )
                service.characteristics = [controlChar, dataChar]
                peripheral.add(service)
            }
            stateEventSink?("poweredOn")
        case .poweredOff:
            stateEventSink?("poweredOff")
        case .unauthorized:
            stateEventSink?("unauthorized")
        case .unsupported:
            stateEventSink?("unsupported")
        default:
            stateEventSink?("unknown")
        }
    }
    
    func peripheralManager(_ peripheral: CBPeripheralManager, didAdd service: CBService, error: Error?) {
        if let error = error {
            NSLog("[UOT_BLE] Failed to add service: \(error.localizedDescription)")
        } else {
            NSLog("[UOT_BLE] GATT service added: \(service.uuid)")
        }
    }
    
    func peripheralManagerDidStartAdvertising(_ peripheral: CBPeripheralManager, error: Error?) {
        if error == nil {
            stateEventSink?("advertising")
        }
    }
}

extension BleAdapterPlugin: CBCentralManagerDelegate {
    func centralManagerDidUpdateState(_ central: CBCentralManager) {
        // State handled by peripheral manager
    }
    
    func centralManager(_ central: CBCentralManager, didDiscover peripheral: CBPeripheral,
                        advertisementData: [String: Any], rssi RSSI: NSNumber) {
        BleAdapterScanStreamHandler.shared.scanEventSink?([
            "id": peripheral.identifier.uuidString,
            "name": peripheral.name ?? "Unknown",
            "rssi": RSSI.intValue
        ])
    }
}

class BleAdapterScanStreamHandler: NSObject, FlutterStreamHandler {
    static let shared = BleAdapterScanStreamHandler()
    var scanEventSink: FlutterEventSink?
    
    func onListen(withArguments arguments: Any?, eventSink events: @escaping FlutterEventSink) -> FlutterError? {
        scanEventSink = events
        return nil
    }
    
    func onCancel(withArguments arguments: Any?) -> FlutterError? {
        scanEventSink = nil
        return nil
    }
}
