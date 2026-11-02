#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BluetoothAdapter(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    BluetoothAdapter,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl BluetoothAdapter {
    pub fn GetDeviceSelector() -> windows_core::Result<String> {
        Self::IBluetoothAdapterStatics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).GetDeviceSelector)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| {
                let hstring: windows_core::HSTRING = core::mem::transmute(result__);
                hstring.to_string_lossy()
            })
        })
    }
    pub fn GetDefaultAsync() -> windows_core::Result<windows_future::IAsyncOperation<Self>> {
        Self::IBluetoothAdapterStatics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).GetDefaultAsync)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }
    fn IBluetoothAdapterStatics<
        R,
        F: FnOnce(&IBluetoothAdapterStatics) -> windows_core::Result<R>,
    >(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<BluetoothAdapter, IBluetoothAdapterStatics> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for BluetoothAdapter {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IBluetoothAdapter>();
}
unsafe impl windows_core::Interface for BluetoothAdapter {
    type Vtable = <IBluetoothAdapter as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID = <IBluetoothAdapter as windows_core::Interface>::IID;
}
impl core::ops::Deref for BluetoothAdapter {
    type Target = IBluetoothAdapter;
    fn deref(&self) -> &Self::Target {
        unsafe { core::mem::transmute(self) }
    }
}
impl windows_core::RuntimeName for BluetoothAdapter {
    const NAME: &'static str = "Windows.Devices.Bluetooth.BluetoothAdapter";
}
unsafe impl Send for BluetoothAdapter {}
unsafe impl Sync for BluetoothAdapter {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BluetoothDevice(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    BluetoothDevice,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl BluetoothDevice {
    pub fn GetDeviceSelectorFromPairingState(pairingstate: bool) -> windows_core::Result<String> {
        Self::IBluetoothDeviceStatics2(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).GetDeviceSelectorFromPairingState)(
                windows_core::Interface::as_raw(this),
                pairingstate,
                &mut result__,
            )
            .map(|| {
                let hstring: windows_core::HSTRING = core::mem::transmute(result__);
                hstring.to_string_lossy()
            })
        })
    }
    fn IBluetoothDeviceStatics2<
        R,
        F: FnOnce(&IBluetoothDeviceStatics2) -> windows_core::Result<R>,
    >(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<BluetoothDevice, IBluetoothDeviceStatics2> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for BluetoothDevice {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IBluetoothDevice>();
}
unsafe impl windows_core::Interface for BluetoothDevice {
    type Vtable = <IBluetoothDevice as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID = <IBluetoothDevice as windows_core::Interface>::IID;
}
impl core::ops::Deref for BluetoothDevice {
    type Target = IBluetoothDevice;
    fn deref(&self) -> &Self::Target {
        unsafe { core::mem::transmute(self) }
    }
}
impl windows_core::RuntimeName for BluetoothDevice {
    const NAME: &'static str = "Windows.Devices.Bluetooth.BluetoothDevice";
}
unsafe impl Send for BluetoothDevice {}
unsafe impl Sync for BluetoothDevice {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BluetoothLEDevice(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    BluetoothLEDevice,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl BluetoothLEDevice {
    pub fn GetDeviceSelectorFromPairingState(pairingstate: bool) -> windows_core::Result<String> {
        Self::IBluetoothLEDeviceStatics2(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).GetDeviceSelectorFromPairingState)(
                windows_core::Interface::as_raw(this),
                pairingstate,
                &mut result__,
            )
            .map(|| {
                let hstring: windows_core::HSTRING = core::mem::transmute(result__);
                hstring.to_string_lossy()
            })
        })
    }
    fn IBluetoothLEDeviceStatics2<
        R,
        F: FnOnce(&IBluetoothLEDeviceStatics2) -> windows_core::Result<R>,
    >(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<
            BluetoothLEDevice,
            IBluetoothLEDeviceStatics2,
        > = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for BluetoothLEDevice {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IBluetoothLEDevice>();
}
unsafe impl windows_core::Interface for BluetoothLEDevice {
    type Vtable = <IBluetoothLEDevice as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID = <IBluetoothLEDevice as windows_core::Interface>::IID;
}
impl core::ops::Deref for BluetoothLEDevice {
    type Target = IBluetoothLEDevice;
    fn deref(&self) -> &Self::Target {
        unsafe { core::mem::transmute(self) }
    }
}
impl windows_core::RuntimeName for BluetoothLEDevice {
    const NAME: &'static str = "Windows.Devices.Bluetooth.BluetoothLEDevice";
}
unsafe impl Send for BluetoothLEDevice {}
unsafe impl Sync for BluetoothLEDevice {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceInformation(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    DeviceInformation,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl DeviceInformation {
    pub fn CreateFromIdAsync(
        deviceid: &str,
    ) -> windows_core::Result<windows_future::IAsyncOperation<Self>> {
        Self::IDeviceInformationStatics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateFromIdAsync)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(&windows_core::HSTRING::from(deviceid)),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }
    pub fn FindAllAsyncWithKindAqsFilterAndAdditionalProperties<P1>(
        aqsfilter: &str,
        additionalproperties: P1,
        kind: DeviceInformationKind,
    ) -> windows_core::Result<windows_future::IAsyncOperation<DeviceInformationCollection>>
    where
        P1: windows_core::Param<windows_collections::IIterable<windows_core::HSTRING>>,
    {
        Self::IDeviceInformationStatics2(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this)
                .FindAllAsyncWithKindAqsFilterAndAdditionalProperties)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(&windows_core::HSTRING::from(aqsfilter)),
                additionalproperties.param().abi(),
                kind,
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }
    pub fn CreateWatcherWithKindAqsFilterAndAdditionalProperties<P1>(
        aqsfilter: &str,
        additionalproperties: P1,
        kind: DeviceInformationKind,
    ) -> windows_core::Result<DeviceWatcher>
    where
        P1: windows_core::Param<windows_collections::IIterable<windows_core::HSTRING>>,
    {
        Self::IDeviceInformationStatics2(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this)
                .CreateWatcherWithKindAqsFilterAndAdditionalProperties)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(&windows_core::HSTRING::from(aqsfilter)),
                additionalproperties.param().abi(),
                kind,
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }
    fn IDeviceInformationStatics<
        R,
        F: FnOnce(&IDeviceInformationStatics) -> windows_core::Result<R>,
    >(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<
            DeviceInformation,
            IDeviceInformationStatics,
        > = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
    fn IDeviceInformationStatics2<
        R,
        F: FnOnce(&IDeviceInformationStatics2) -> windows_core::Result<R>,
    >(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<
            DeviceInformation,
            IDeviceInformationStatics2,
        > = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for DeviceInformation {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IDeviceInformation>();
}
unsafe impl windows_core::Interface for DeviceInformation {
    type Vtable = <IDeviceInformation as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID = <IDeviceInformation as windows_core::Interface>::IID;
}
impl core::ops::Deref for DeviceInformation {
    type Target = IDeviceInformation;
    fn deref(&self) -> &Self::Target {
        unsafe { core::mem::transmute(self) }
    }
}
impl windows_core::RuntimeName for DeviceInformation {
    const NAME: &'static str = "Windows.Devices.Enumeration.DeviceInformation";
}
unsafe impl Send for DeviceInformation {}
unsafe impl Sync for DeviceInformation {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceInformationCollection(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    DeviceInformationCollection,
    windows_core::IUnknown,
    windows_core::IInspectable,
    windows_collections::IVectorView<DeviceInformation>
);
impl windows_core::RuntimeType for DeviceInformationCollection {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<
        Self,
        windows_collections::IVectorView<DeviceInformation>,
    >();
}
unsafe impl windows_core::Interface for DeviceInformationCollection {
    type Vtable =
        <windows_collections::IVectorView<DeviceInformation> as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID =
        <windows_collections::IVectorView<DeviceInformation> as windows_core::Interface>::IID;
}
impl core::ops::Deref for DeviceInformationCollection {
    type Target = windows_collections::IVectorView<DeviceInformation>;
    fn deref(&self) -> &Self::Target {
        unsafe { core::mem::transmute(self) }
    }
}
impl windows_core::RuntimeName for DeviceInformationCollection {
    const NAME: &'static str = "Windows.Devices.Enumeration.DeviceInformationCollection";
}
unsafe impl Send for DeviceInformationCollection {}
unsafe impl Sync for DeviceInformationCollection {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceInformationCustomPairing(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    DeviceInformationCustomPairing,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl windows_core::RuntimeType for DeviceInformationCustomPairing {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IDeviceInformationCustomPairing>();
}
unsafe impl windows_core::Interface for DeviceInformationCustomPairing {
    type Vtable = <IDeviceInformationCustomPairing as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID =
        <IDeviceInformationCustomPairing as windows_core::Interface>::IID;
}
impl core::ops::Deref for DeviceInformationCustomPairing {
    type Target = IDeviceInformationCustomPairing;
    fn deref(&self) -> &Self::Target {
        unsafe { core::mem::transmute(self) }
    }
}
impl windows_core::RuntimeName for DeviceInformationCustomPairing {
    const NAME: &'static str = "Windows.Devices.Enumeration.DeviceInformationCustomPairing";
}
unsafe impl Send for DeviceInformationCustomPairing {}
unsafe impl Sync for DeviceInformationCustomPairing {}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DeviceInformationKind(pub i32);
impl DeviceInformationKind {
    pub const Unknown: Self = Self(0);
    pub const DeviceInterface: Self = Self(1);
    pub const DeviceContainer: Self = Self(2);
    pub const Device: Self = Self(3);
    pub const DeviceInterfaceClass: Self = Self(4);
    pub const AssociationEndpoint: Self = Self(5);
    pub const AssociationEndpointContainer: Self = Self(6);
    pub const AssociationEndpointService: Self = Self(7);
    pub const DevicePanel: Self = Self(8);
    pub const AssociationEndpointProtocol: Self = Self(9);
}
impl windows_core::imp::TypeKind for DeviceInformationKind {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for DeviceInformationKind {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(
        b"enum(Windows.Devices.Enumeration.DeviceInformationKind;i4)",
    );
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceInformationPairing(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    DeviceInformationPairing,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl windows_core::RuntimeType for DeviceInformationPairing {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IDeviceInformationPairing>();
}
unsafe impl windows_core::Interface for DeviceInformationPairing {
    type Vtable = <IDeviceInformationPairing as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID = <IDeviceInformationPairing as windows_core::Interface>::IID;
}
impl core::ops::Deref for DeviceInformationPairing {
    type Target = IDeviceInformationPairing;
    fn deref(&self) -> &Self::Target {
        unsafe { core::mem::transmute(self) }
    }
}
impl windows_core::RuntimeName for DeviceInformationPairing {
    const NAME: &'static str = "Windows.Devices.Enumeration.DeviceInformationPairing";
}
unsafe impl Send for DeviceInformationPairing {}
unsafe impl Sync for DeviceInformationPairing {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceInformationUpdate(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    DeviceInformationUpdate,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl windows_core::RuntimeType for DeviceInformationUpdate {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IDeviceInformationUpdate>();
}
unsafe impl windows_core::Interface for DeviceInformationUpdate {
    type Vtable = <IDeviceInformationUpdate as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID = <IDeviceInformationUpdate as windows_core::Interface>::IID;
}
impl core::ops::Deref for DeviceInformationUpdate {
    type Target = IDeviceInformationUpdate;
    fn deref(&self) -> &Self::Target {
        unsafe { core::mem::transmute(self) }
    }
}
impl windows_core::RuntimeName for DeviceInformationUpdate {
    const NAME: &'static str = "Windows.Devices.Enumeration.DeviceInformationUpdate";
}
unsafe impl Send for DeviceInformationUpdate {}
unsafe impl Sync for DeviceInformationUpdate {}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DevicePairingKinds(pub u32);
impl DevicePairingKinds {
    pub const None: Self = Self(0);
    pub const ConfirmOnly: Self = Self(1);
    pub const DisplayPin: Self = Self(2);
    pub const ProvidePin: Self = Self(4);
    pub const ConfirmPinMatch: Self = Self(8);
    pub const ProvidePasswordCredential: Self = Self(16);
    pub const ProvideAddress: Self = Self(32);
}
impl windows_core::imp::TypeKind for DevicePairingKinds {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for DevicePairingKinds {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(
        b"enum(Windows.Devices.Enumeration.DevicePairingKinds;u4)",
    );
}
impl DevicePairingKinds {
    pub const fn contains(&self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}
impl core::ops::BitOr for DevicePairingKinds {
    type Output = Self;
    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}
impl core::ops::BitAnd for DevicePairingKinds {
    type Output = Self;
    fn bitand(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}
impl core::ops::BitOrAssign for DevicePairingKinds {
    fn bitor_assign(&mut self, other: Self) {
        self.0.bitor_assign(other.0);
    }
}
impl core::ops::BitAndAssign for DevicePairingKinds {
    fn bitand_assign(&mut self, other: Self) {
        self.0.bitand_assign(other.0);
    }
}
impl core::ops::Not for DevicePairingKinds {
    type Output = Self;
    fn not(self) -> Self {
        Self(self.0.not())
    }
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DevicePairingRequestedEventArgs(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    DevicePairingRequestedEventArgs,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl windows_core::RuntimeType for DevicePairingRequestedEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IDevicePairingRequestedEventArgs>();
}
unsafe impl windows_core::Interface for DevicePairingRequestedEventArgs {
    type Vtable = <IDevicePairingRequestedEventArgs as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID =
        <IDevicePairingRequestedEventArgs as windows_core::Interface>::IID;
}
impl core::ops::Deref for DevicePairingRequestedEventArgs {
    type Target = IDevicePairingRequestedEventArgs;
    fn deref(&self) -> &Self::Target {
        unsafe { core::mem::transmute(self) }
    }
}
impl windows_core::RuntimeName for DevicePairingRequestedEventArgs {
    const NAME: &'static str = "Windows.Devices.Enumeration.DevicePairingRequestedEventArgs";
}
unsafe impl Send for DevicePairingRequestedEventArgs {}
unsafe impl Sync for DevicePairingRequestedEventArgs {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DevicePairingResult(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    DevicePairingResult,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl windows_core::RuntimeType for DevicePairingResult {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IDevicePairingResult>();
}
unsafe impl windows_core::Interface for DevicePairingResult {
    type Vtable = <IDevicePairingResult as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID = <IDevicePairingResult as windows_core::Interface>::IID;
}
impl core::ops::Deref for DevicePairingResult {
    type Target = IDevicePairingResult;
    fn deref(&self) -> &Self::Target {
        unsafe { core::mem::transmute(self) }
    }
}
impl windows_core::RuntimeName for DevicePairingResult {
    const NAME: &'static str = "Windows.Devices.Enumeration.DevicePairingResult";
}
unsafe impl Send for DevicePairingResult {}
unsafe impl Sync for DevicePairingResult {}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DevicePairingResultStatus(pub i32);
impl DevicePairingResultStatus {
    pub const Paired: Self = Self(0);
    pub const NotReadyToPair: Self = Self(1);
    pub const NotPaired: Self = Self(2);
    pub const AlreadyPaired: Self = Self(3);
    pub const ConnectionRejected: Self = Self(4);
    pub const TooManyConnections: Self = Self(5);
    pub const HardwareFailure: Self = Self(6);
    pub const AuthenticationTimeout: Self = Self(7);
    pub const AuthenticationNotAllowed: Self = Self(8);
    pub const AuthenticationFailure: Self = Self(9);
    pub const NoSupportedProfiles: Self = Self(10);
    pub const ProtectionLevelCouldNotBeMet: Self = Self(11);
    pub const AccessDenied: Self = Self(12);
    pub const InvalidCeremonyData: Self = Self(13);
    pub const PairingCanceled: Self = Self(14);
    pub const OperationAlreadyInProgress: Self = Self(15);
    pub const RequiredHandlerNotRegistered: Self = Self(16);
    pub const RejectedByHandler: Self = Self(17);
    pub const RemoteDeviceHasAssociation: Self = Self(18);
    pub const Failed: Self = Self(19);
}
impl windows_core::imp::TypeKind for DevicePairingResultStatus {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for DevicePairingResultStatus {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(
        b"enum(Windows.Devices.Enumeration.DevicePairingResultStatus;i4)",
    );
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceUnpairingResult(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    DeviceUnpairingResult,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl windows_core::RuntimeType for DeviceUnpairingResult {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IDeviceUnpairingResult>();
}
unsafe impl windows_core::Interface for DeviceUnpairingResult {
    type Vtable = <IDeviceUnpairingResult as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID = <IDeviceUnpairingResult as windows_core::Interface>::IID;
}
impl core::ops::Deref for DeviceUnpairingResult {
    type Target = IDeviceUnpairingResult;
    fn deref(&self) -> &Self::Target {
        unsafe { core::mem::transmute(self) }
    }
}
impl windows_core::RuntimeName for DeviceUnpairingResult {
    const NAME: &'static str = "Windows.Devices.Enumeration.DeviceUnpairingResult";
}
unsafe impl Send for DeviceUnpairingResult {}
unsafe impl Sync for DeviceUnpairingResult {}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DeviceUnpairingResultStatus(pub i32);
impl DeviceUnpairingResultStatus {
    pub const Unpaired: Self = Self(0);
    pub const AlreadyUnpaired: Self = Self(1);
    pub const OperationAlreadyInProgress: Self = Self(2);
    pub const AccessDenied: Self = Self(3);
    pub const Failed: Self = Self(4);
}
impl windows_core::imp::TypeKind for DeviceUnpairingResultStatus {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for DeviceUnpairingResultStatus {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(
        b"enum(Windows.Devices.Enumeration.DeviceUnpairingResultStatus;i4)",
    );
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceWatcher(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    DeviceWatcher,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl windows_core::RuntimeType for DeviceWatcher {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IDeviceWatcher>();
}
unsafe impl windows_core::Interface for DeviceWatcher {
    type Vtable = <IDeviceWatcher as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID = <IDeviceWatcher as windows_core::Interface>::IID;
}
impl core::ops::Deref for DeviceWatcher {
    type Target = IDeviceWatcher;
    fn deref(&self) -> &Self::Target {
        unsafe { core::mem::transmute(self) }
    }
}
impl windows_core::RuntimeName for DeviceWatcher {
    const NAME: &'static str = "Windows.Devices.Enumeration.DeviceWatcher";
}
unsafe impl Send for DeviceWatcher {}
unsafe impl Sync for DeviceWatcher {}
windows_core::imp::define_interface!(
    IBluetoothAdapter,
    IBluetoothAdapter_Vtbl,
    0x7974f04c_5f7a_4a34_9225_a855f84b1a8b
);
impl windows_core::RuntimeType for IBluetoothAdapter {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
impl IBluetoothAdapter {
    pub fn BluetoothAddress(&self) -> windows_core::Result<u64> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).BluetoothAddress)(
                windows_core::Interface::as_raw(self),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn GetRadioAsync(&self) -> windows_core::Result<windows_future::IAsyncOperation<Radio>> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).GetRadioAsync)(
                windows_core::Interface::as_raw(self),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }
}
#[repr(C)]
pub struct IBluetoothAdapter_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    DeviceId: usize,
    pub BluetoothAddress:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut u64) -> windows_core::HRESULT,
    IsClassicSupported: usize,
    IsLowEnergySupported: usize,
    IsPeripheralRoleSupported: usize,
    IsCentralRoleSupported: usize,
    IsAdvertisementOffloadSupported: usize,
    pub GetRadioAsync: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IBluetoothAdapterStatics,
    IBluetoothAdapterStatics_Vtbl,
    0x8b02fb6a_ac4c_4741_8661_8eab7d17ea9f
);
impl windows_core::RuntimeType for IBluetoothAdapterStatics {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IBluetoothAdapterStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub GetDeviceSelector: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    FromIdAsync: usize,
    pub GetDefaultAsync: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IBluetoothDevice,
    IBluetoothDevice_Vtbl,
    0x2335b156_90d2_4a04_aef5_0e20b9e6b707
);
impl windows_core::RuntimeType for IBluetoothDevice {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IBluetoothDevice_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(
    IBluetoothDeviceStatics2,
    IBluetoothDeviceStatics2_Vtbl,
    0xc29e8e2f_4e14_4477_aa1b_b8b47e5b7ece
);
impl windows_core::RuntimeType for IBluetoothDeviceStatics2 {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IBluetoothDeviceStatics2_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub GetDeviceSelectorFromPairingState: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        bool,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IBluetoothLEDevice,
    IBluetoothLEDevice_Vtbl,
    0xb5ee2f7b_4ad8_4642_ac48_80a0b500e887
);
impl windows_core::RuntimeType for IBluetoothLEDevice {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IBluetoothLEDevice_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(
    IBluetoothLEDeviceStatics2,
    IBluetoothLEDeviceStatics2_Vtbl,
    0x5f12c06b_3bac_43e8_ad16_563271bd41c2
);
impl windows_core::RuntimeType for IBluetoothLEDeviceStatics2 {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IBluetoothLEDeviceStatics2_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub GetDeviceSelectorFromPairingState: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        bool,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IDeviceInformation,
    IDeviceInformation_Vtbl,
    0xaba0fb95_4398_489d_8e44_e6130927011f
);
impl windows_core::RuntimeType for IDeviceInformation {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
impl IDeviceInformation {
    pub fn Id(&self) -> windows_core::Result<String> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Id)(
                windows_core::Interface::as_raw(self),
                &mut result__,
            )
            .map(|| {
                let hstring: windows_core::HSTRING = core::mem::transmute(result__);
                hstring.to_string_lossy()
            })
        }
    }
    pub fn Name(&self) -> windows_core::Result<String> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Name)(
                windows_core::Interface::as_raw(self),
                &mut result__,
            )
            .map(|| {
                let hstring: windows_core::HSTRING = core::mem::transmute(result__);
                hstring.to_string_lossy()
            })
        }
    }
    pub fn Properties(
        &self,
    ) -> windows_core::Result<
        windows_collections::IMapView<windows_core::HSTRING, windows_core::IInspectable>,
    > {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Properties)(
                windows_core::Interface::as_raw(self),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }
}
#[repr(C)]
pub struct IDeviceInformation_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Id: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub Name: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    IsEnabled: usize,
    IsDefault: usize,
    EnclosureLocation: usize,
    pub Properties: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IDeviceInformation2,
    IDeviceInformation2_Vtbl,
    0xf156a638_7997_48d9_a10c_269d46533f48
);
impl windows_core::RuntimeType for IDeviceInformation2 {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
impl IDeviceInformation2 {
    pub fn Pairing(&self) -> windows_core::Result<DeviceInformationPairing> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Pairing)(
                windows_core::Interface::as_raw(self),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }
}
#[repr(C)]
pub struct IDeviceInformation2_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    Kind: usize,
    pub Pairing: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IDeviceInformationCustomPairing,
    IDeviceInformationCustomPairing_Vtbl,
    0x85138c02_4ee6_4914_8370_107a39144c0e
);
impl windows_core::RuntimeType for IDeviceInformationCustomPairing {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
impl IDeviceInformationCustomPairing {
    pub fn PairAsync(
        &self,
        pairingkindssupported: DevicePairingKinds,
    ) -> windows_core::Result<windows_future::IAsyncOperation<DevicePairingResult>> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).PairAsync)(
                windows_core::Interface::as_raw(self),
                pairingkindssupported,
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }
    pub fn PairingRequested<F>(
        &self,
        handler: F,
    ) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(
                windows_core::Ref<DeviceInformationCustomPairing>,
                windows_core::Ref<DevicePairingRequestedEventArgs>,
            ) + 'static,
    {
        let handler: TypedEventHandler<
            DeviceInformationCustomPairing,
            DevicePairingRequestedEventArgs,
        > = {
            let com = windows_core::imp::DelegateBox::<
                TypedEventHandler<DeviceInformationCustomPairing, DevicePairingRequestedEventArgs>,
                F,
            >::new(
                &TypedEventHandlerBox::<
                    DeviceInformationCustomPairing,
                    DevicePairingRequestedEventArgs,
                    F,
                >::VTABLE,
                handler,
            );
            unsafe { core::mem::transmute(windows_core::imp::box_new(com)) }
        };
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).PairingRequested)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(
                self.clone(),
                token__,
                windows_core::Interface::vtable(self).RemovePairingRequested,
            ))
        }
    }
}
#[repr(C)]
pub struct IDeviceInformationCustomPairing_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub PairAsync: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        DevicePairingKinds,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    PairWithProtectionLevelAsync: usize,
    PairWithProtectionLevelAndSettingsAsync: usize,
    pub PairingRequested: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut i64,
    ) -> windows_core::HRESULT,
    pub RemovePairingRequested:
        unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IDeviceInformationPairing,
    IDeviceInformationPairing_Vtbl,
    0x2c4769f5_f684_40d5_8469_e8dbaab70485
);
impl windows_core::RuntimeType for IDeviceInformationPairing {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IDeviceInformationPairing_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(
    IDeviceInformationPairing2,
    IDeviceInformationPairing2_Vtbl,
    0xf68612fd_0aee_4328_85cc_1c742bb1790d
);
impl windows_core::RuntimeType for IDeviceInformationPairing2 {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
impl IDeviceInformationPairing2 {
    pub fn Custom(&self) -> windows_core::Result<DeviceInformationCustomPairing> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Custom)(
                windows_core::Interface::as_raw(self),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }
    pub fn UnpairAsync(
        &self,
    ) -> windows_core::Result<windows_future::IAsyncOperation<DeviceUnpairingResult>> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).UnpairAsync)(
                windows_core::Interface::as_raw(self),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }
}
#[repr(C)]
pub struct IDeviceInformationPairing2_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    ProtectionLevel: usize,
    pub Custom: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    PairWithProtectionLevelAndSettingsAsync: usize,
    pub UnpairAsync: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IDeviceInformationStatics,
    IDeviceInformationStatics_Vtbl,
    0xc17f100e_3a46_4a78_8013_769dc9b97390
);
impl windows_core::RuntimeType for IDeviceInformationStatics {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IDeviceInformationStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateFromIdAsync: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IDeviceInformationStatics2,
    IDeviceInformationStatics2_Vtbl,
    0x493b4f34_a84f_45fd_9167_15d1cb1bd1f9
);
impl windows_core::RuntimeType for IDeviceInformationStatics2 {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IDeviceInformationStatics2_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    GetAqsFilterFromDeviceClass: usize,
    CreateFromIdAsyncWithKindAndAdditionalProperties: usize,
    pub FindAllAsyncWithKindAqsFilterAndAdditionalProperties:
        unsafe extern "system" fn(
            *mut core::ffi::c_void,
            *mut core::ffi::c_void,
            *mut core::ffi::c_void,
            DeviceInformationKind,
            *mut *mut core::ffi::c_void,
        ) -> windows_core::HRESULT,
    pub CreateWatcherWithKindAqsFilterAndAdditionalProperties:
        unsafe extern "system" fn(
            *mut core::ffi::c_void,
            *mut core::ffi::c_void,
            *mut core::ffi::c_void,
            DeviceInformationKind,
            *mut *mut core::ffi::c_void,
        ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IDeviceInformationUpdate,
    IDeviceInformationUpdate_Vtbl,
    0x8f315305_d972_44b7_a37e_9e822c78213b
);
impl windows_core::RuntimeType for IDeviceInformationUpdate {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
impl IDeviceInformationUpdate {
    pub fn Id(&self) -> windows_core::Result<String> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Id)(
                windows_core::Interface::as_raw(self),
                &mut result__,
            )
            .map(|| {
                let hstring: windows_core::HSTRING = core::mem::transmute(result__);
                hstring.to_string_lossy()
            })
        }
    }
}
#[repr(C)]
pub struct IDeviceInformationUpdate_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Id: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IDevicePairingRequestedEventArgs,
    IDevicePairingRequestedEventArgs_Vtbl,
    0xf717fc56_de6b_487f_8376_0180aca69963
);
impl windows_core::RuntimeType for IDevicePairingRequestedEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
impl IDevicePairingRequestedEventArgs {
    pub fn PairingKind(&self) -> windows_core::Result<DevicePairingKinds> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).PairingKind)(
                windows_core::Interface::as_raw(self),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn Pin(&self) -> windows_core::Result<String> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Pin)(
                windows_core::Interface::as_raw(self),
                &mut result__,
            )
            .map(|| {
                let hstring: windows_core::HSTRING = core::mem::transmute(result__);
                hstring.to_string_lossy()
            })
        }
    }
    pub fn Accept(&self) -> windows_core::Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).Accept)(windows_core::Interface::as_raw(self))
                .ok()
        }
    }
    pub fn AcceptWithPin(&self, pin: &str) -> windows_core::Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).AcceptWithPin)(
                windows_core::Interface::as_raw(self),
                core::mem::transmute_copy(&windows_core::HSTRING::from(pin)),
            )
            .ok()
        }
    }
}
#[repr(C)]
pub struct IDevicePairingRequestedEventArgs_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    DeviceInformation: usize,
    pub PairingKind: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut DevicePairingKinds,
    ) -> windows_core::HRESULT,
    pub Pin: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub Accept: unsafe extern "system" fn(*mut core::ffi::c_void) -> windows_core::HRESULT,
    pub AcceptWithPin: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IDevicePairingResult,
    IDevicePairingResult_Vtbl,
    0x072b02bf_dd95_4025_9b37_de51adba37b7
);
impl windows_core::RuntimeType for IDevicePairingResult {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
impl IDevicePairingResult {
    pub fn Status(&self) -> windows_core::Result<DevicePairingResultStatus> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Status)(
                windows_core::Interface::as_raw(self),
                &mut result__,
            )
            .map(|| result__)
        }
    }
}
#[repr(C)]
pub struct IDevicePairingResult_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Status: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut DevicePairingResultStatus,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IDeviceUnpairingResult,
    IDeviceUnpairingResult_Vtbl,
    0x66f44ad3_79d9_444b_92cf_a92ef72571c7
);
impl windows_core::RuntimeType for IDeviceUnpairingResult {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
impl IDeviceUnpairingResult {
    pub fn Status(&self) -> windows_core::Result<DeviceUnpairingResultStatus> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Status)(
                windows_core::Interface::as_raw(self),
                &mut result__,
            )
            .map(|| result__)
        }
    }
}
#[repr(C)]
pub struct IDeviceUnpairingResult_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Status: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut DeviceUnpairingResultStatus,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IDeviceWatcher,
    IDeviceWatcher_Vtbl,
    0xc9eab97d_8f6b_4f96_a9f4_abc814e22271
);
impl windows_core::RuntimeType for IDeviceWatcher {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
impl IDeviceWatcher {
    pub fn Added<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<DeviceWatcher>, windows_core::Ref<DeviceInformation>) + 'static,
    {
        let handler: TypedEventHandler<DeviceWatcher, DeviceInformation> = {
            let com = windows_core::imp::DelegateBox::<
                TypedEventHandler<DeviceWatcher, DeviceInformation>,
                F,
            >::new(
                &TypedEventHandlerBox::<DeviceWatcher, DeviceInformation, F>::VTABLE,
                handler,
            );
            unsafe { core::mem::transmute(windows_core::imp::box_new(com)) }
        };
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).Added)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(
                self.clone(),
                token__,
                windows_core::Interface::vtable(self).RemoveAdded,
            ))
        }
    }
    pub fn Updated<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<DeviceWatcher>, windows_core::Ref<DeviceInformationUpdate>)
            + 'static,
    {
        let handler: TypedEventHandler<DeviceWatcher, DeviceInformationUpdate> = {
            let com = windows_core::imp::DelegateBox::<
                TypedEventHandler<DeviceWatcher, DeviceInformationUpdate>,
                F,
            >::new(
                &TypedEventHandlerBox::<DeviceWatcher, DeviceInformationUpdate, F>::VTABLE,
                handler,
            );
            unsafe { core::mem::transmute(windows_core::imp::box_new(com)) }
        };
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).Updated)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(
                self.clone(),
                token__,
                windows_core::Interface::vtable(self).RemoveUpdated,
            ))
        }
    }
    pub fn Removed<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<DeviceWatcher>, windows_core::Ref<DeviceInformationUpdate>)
            + 'static,
    {
        let handler: TypedEventHandler<DeviceWatcher, DeviceInformationUpdate> = {
            let com = windows_core::imp::DelegateBox::<
                TypedEventHandler<DeviceWatcher, DeviceInformationUpdate>,
                F,
            >::new(
                &TypedEventHandlerBox::<DeviceWatcher, DeviceInformationUpdate, F>::VTABLE,
                handler,
            );
            unsafe { core::mem::transmute(windows_core::imp::box_new(com)) }
        };
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).Removed)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(
                self.clone(),
                token__,
                windows_core::Interface::vtable(self).RemoveRemoved,
            ))
        }
    }
    pub fn EnumerationCompleted<F>(
        &self,
        handler: F,
    ) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<DeviceWatcher>, windows_core::Ref<windows_core::IInspectable>)
            + 'static,
    {
        let handler: TypedEventHandler<DeviceWatcher, windows_core::IInspectable> = {
            let com = windows_core::imp::DelegateBox::<
                TypedEventHandler<DeviceWatcher, windows_core::IInspectable>,
                F,
            >::new(
                &TypedEventHandlerBox::<DeviceWatcher, windows_core::IInspectable, F>::VTABLE,
                handler,
            );
            unsafe { core::mem::transmute(windows_core::imp::box_new(com)) }
        };
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).EnumerationCompleted)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(
                self.clone(),
                token__,
                windows_core::Interface::vtable(self).RemoveEnumerationCompleted,
            ))
        }
    }
    pub fn Start(&self) -> windows_core::Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).Start)(windows_core::Interface::as_raw(self))
                .ok()
        }
    }
    pub fn Stop(&self) -> windows_core::Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).Stop)(windows_core::Interface::as_raw(self)).ok()
        }
    }
}
#[repr(C)]
pub struct IDeviceWatcher_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Added: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut i64,
    ) -> windows_core::HRESULT,
    pub RemoveAdded:
        unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
    pub Updated: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut i64,
    ) -> windows_core::HRESULT,
    pub RemoveUpdated:
        unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
    pub Removed: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut i64,
    ) -> windows_core::HRESULT,
    pub RemoveRemoved:
        unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
    pub EnumerationCompleted: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut i64,
    ) -> windows_core::HRESULT,
    pub RemoveEnumerationCompleted:
        unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
    Stopped: usize,
    RemoveStopped: usize,
    Status: usize,
    pub Start: unsafe extern "system" fn(*mut core::ffi::c_void) -> windows_core::HRESULT,
    pub Stop: unsafe extern "system" fn(*mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IPropertyValue,
    IPropertyValue_Vtbl,
    0x4bd682dd_7554_40e9_9a9b_82654ede7e62
);
impl windows_core::RuntimeType for IPropertyValue {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
windows_core::imp::interface_hierarchy!(
    IPropertyValue,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl windows_core::RuntimeName for IPropertyValue {
    const NAME: &'static str = "Windows.Foundation.IPropertyValue";
}
#[repr(C)]
pub struct IPropertyValue_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IRadio, IRadio_Vtbl, 0x252118df_b33e_416a_875f_1cf38ae2d83e);
impl windows_core::RuntimeType for IRadio {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
impl IRadio {
    pub fn SetStateAsync(
        &self,
        value: RadioState,
    ) -> windows_core::Result<windows_future::IAsyncOperation<RadioAccessStatus>> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).SetStateAsync)(
                windows_core::Interface::as_raw(self),
                value,
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }
    pub fn StateChanged<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<Radio>, windows_core::Ref<windows_core::IInspectable>) + 'static,
    {
        let handler: TypedEventHandler<Radio, windows_core::IInspectable> = {
            let com = windows_core::imp::DelegateBox::<
                TypedEventHandler<Radio, windows_core::IInspectable>,
                F,
            >::new(
                &TypedEventHandlerBox::<Radio, windows_core::IInspectable, F>::VTABLE,
                handler,
            );
            unsafe { core::mem::transmute(windows_core::imp::box_new(com)) }
        };
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).StateChanged)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(
                self.clone(),
                token__,
                windows_core::Interface::vtable(self).RemoveStateChanged,
            ))
        }
    }
    pub fn State(&self) -> windows_core::Result<RadioState> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).State)(
                windows_core::Interface::as_raw(self),
                &mut result__,
            )
            .map(|| result__)
        }
    }
}
#[repr(C)]
pub struct IRadio_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub SetStateAsync: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        RadioState,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub StateChanged: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut i64,
    ) -> windows_core::HRESULT,
    pub RemoveStateChanged:
        unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
    pub State:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut RadioState) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IRadioStatics,
    IRadioStatics_Vtbl,
    0x5fb6a12e_67cb_46ae_aae9_65919f86eff4
);
impl windows_core::RuntimeType for IRadioStatics {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IRadioStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    GetRadiosAsync: usize,
    GetDeviceSelector: usize,
    FromIdAsync: usize,
    pub RequestAccessAsync: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IReference<T>(windows_core::IUnknown, core::marker::PhantomData<T>)
where
    T: windows_core::RuntimeType + 'static;
impl<T: windows_core::RuntimeType + 'static> windows_core::imp::CanInto<windows_core::IUnknown>
    for IReference<T>
{
}
impl<T: windows_core::RuntimeType + 'static> windows_core::imp::CanInto<windows_core::IInspectable>
    for IReference<T>
{
}
unsafe impl<T: windows_core::RuntimeType + 'static> windows_core::Interface for IReference<T> {
    type Vtable = IReference_Vtbl<T>;
    const IID: windows_core::GUID =
        windows_core::GUID::from_signature(<Self as windows_core::RuntimeType>::SIGNATURE);
}
impl<T: windows_core::RuntimeType + 'static> windows_core::RuntimeType for IReference<T> {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::new()
        .push_slice(b"pinterface({61c17706-2d65-11e0-9ae8-d48564015472}")
        .push_slice(b";")
        .push_other(T::SIGNATURE)
        .push_slice(b")");
}
impl<T: windows_core::RuntimeType + 'static> windows_core::imp::CanInto<IPropertyValue>
    for IReference<T>
{
    const QUERY: bool = true;
}
impl<T: windows_core::RuntimeType + 'static> IReference<T> {
    pub fn Value(&self) -> windows_core::Result<T> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Value)(
                windows_core::Interface::as_raw(self),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }
}
impl<T: windows_core::RuntimeType + 'static> windows_core::RuntimeName for IReference<T> {
    const NAME: &'static str = "Windows.Foundation.IReference";
    const RUNTIME_CLASS_NAME: windows_core::imp::ConstBuffer =
        <Self as windows_core::RuntimeType>::NAME;
}
#[repr(C)]
pub struct IReference_Vtbl<T>
where
    T: windows_core::RuntimeType + 'static,
{
    pub base__: windows_core::IInspectable_Vtbl,
    pub Value: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut windows_core::imp::AbiType<T>,
    ) -> windows_core::HRESULT,
    T: core::marker::PhantomData<T>,
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Radio(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(Radio, windows_core::IUnknown, windows_core::IInspectable);
impl Radio {
    pub fn RequestAccessAsync()
    -> windows_core::Result<windows_future::IAsyncOperation<RadioAccessStatus>> {
        Self::IRadioStatics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).RequestAccessAsync)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }
    fn IRadioStatics<R, F: FnOnce(&IRadioStatics) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Radio, IRadioStatics> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for Radio {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IRadio>();
}
unsafe impl windows_core::Interface for Radio {
    type Vtable = <IRadio as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID = <IRadio as windows_core::Interface>::IID;
}
impl core::ops::Deref for Radio {
    type Target = IRadio;
    fn deref(&self) -> &Self::Target {
        unsafe { core::mem::transmute(self) }
    }
}
impl windows_core::RuntimeName for Radio {
    const NAME: &'static str = "Windows.Devices.Radios.Radio";
}
unsafe impl Send for Radio {}
unsafe impl Sync for Radio {}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RadioAccessStatus(pub i32);
impl RadioAccessStatus {
    pub const Unspecified: Self = Self(0);
    pub const Allowed: Self = Self(1);
    pub const DeniedByUser: Self = Self(2);
    pub const DeniedBySystem: Self = Self(3);
}
impl windows_core::imp::TypeKind for RadioAccessStatus {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for RadioAccessStatus {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(
        b"enum(Windows.Devices.Radios.RadioAccessStatus;i4)",
    );
}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RadioState(pub i32);
impl RadioState {
    pub const Unknown: Self = Self(0);
    pub const On: Self = Self(1);
    pub const Off: Self = Self(2);
    pub const Disabled: Self = Self(3);
}
impl windows_core::imp::TypeKind for RadioState {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for RadioState {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"enum(Windows.Devices.Radios.RadioState;i4)");
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypedEventHandler<TSender, TResult>(
    windows_core::IUnknown,
    core::marker::PhantomData<TSender>,
    core::marker::PhantomData<TResult>,
)
where
    TSender: windows_core::RuntimeType + 'static,
    TResult: windows_core::RuntimeType + 'static;
unsafe impl<
    TSender: windows_core::RuntimeType + 'static,
    TResult: windows_core::RuntimeType + 'static,
> windows_core::Interface for TypedEventHandler<TSender, TResult>
{
    type Vtable = TypedEventHandler_Vtbl<TSender, TResult>;
    const IID: windows_core::GUID =
        windows_core::GUID::from_signature(<Self as windows_core::RuntimeType>::SIGNATURE);
}
impl<TSender: windows_core::RuntimeType + 'static, TResult: windows_core::RuntimeType + 'static>
    windows_core::RuntimeType for TypedEventHandler<TSender, TResult>
{
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::new()
        .push_slice(b"pinterface({9de1c534-6ae1-11e0-84e1-18a905bcc53f}")
        .push_slice(b";")
        .push_other(TSender::SIGNATURE)
        .push_slice(b";")
        .push_other(TResult::SIGNATURE)
        .push_slice(b")");
}
#[repr(C)]
pub struct TypedEventHandler_Vtbl<TSender, TResult>
where
    TSender: windows_core::RuntimeType + 'static,
    TResult: windows_core::RuntimeType + 'static,
{
    base__: windows_core::IUnknown_Vtbl,
    Invoke: unsafe extern "system" fn(
        this: *mut core::ffi::c_void,
        sender: windows_core::imp::AbiType<TSender>,
        args: windows_core::imp::AbiType<TResult>,
    ) -> windows_core::HRESULT,
    TSender: core::marker::PhantomData<TSender>,
    TResult: core::marker::PhantomData<TResult>,
}
struct TypedEventHandlerBox<
    TSender,
    TResult,
    F: Fn(windows_core::Ref<TSender>, windows_core::Ref<TResult>) + 'static,
>(core::marker::PhantomData<(TSender, TResult, fn() -> F)>)
where
    TSender: windows_core::RuntimeType + 'static,
    TResult: windows_core::RuntimeType + 'static;
impl<
    TSender: windows_core::RuntimeType + 'static,
    TResult: windows_core::RuntimeType + 'static,
    F: Fn(windows_core::Ref<TSender>, windows_core::Ref<TResult>) + 'static,
> TypedEventHandlerBox<TSender, TResult, F>
{
    const VTABLE : TypedEventHandler_Vtbl < TSender , TResult , > = TypedEventHandler_Vtbl::< TSender , TResult , > { base__ : windows_core::IUnknown_Vtbl { QueryInterface : windows_core::imp::DelegateBox::< TypedEventHandler < TSender , TResult > , F >::QueryInterface , AddRef : windows_core::imp::DelegateBox::< TypedEventHandler < TSender , TResult > , F >::AddRef , Release : windows_core::imp::DelegateBox::< TypedEventHandler < TSender , TResult > , F >::Release , } , Invoke : Self::Invoke , TSender : core::marker::PhantomData::< TSender > , TResult : core::marker::PhantomData::< TResult > } ;
    unsafe extern "system" fn Invoke(
        this: *mut core::ffi::c_void,
        sender: windows_core::imp::AbiType<TSender>,
        args: windows_core::imp::AbiType<TResult>,
    ) -> windows_core::HRESULT {
        unsafe {
            let this = &mut *(this as *mut *mut core::ffi::c_void
                as *mut windows_core::imp::DelegateBox<TypedEventHandler<TSender, TResult>, F>);
            (this.invoke)(
                core::mem::transmute_copy(&sender),
                core::mem::transmute_copy(&args),
            );
            windows_core::HRESULT(0)
        }
    }
}
