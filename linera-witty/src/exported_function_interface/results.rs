use {
    crate::{
        memory_layout::FlatLayout,
        primitive_types::{FlatType, MaybeFlatType},
        GuestPointer, InstanceWithMemory, Layout, Memory, Runtime, RuntimeError, RuntimeMemory,
        WitStore,
    },
    frunk::{HList, HNil},
};

pub trait FunctionResult: WitStore {
    type ExtraParameter: FlatLayout;
    type Output: MaybeFlatType;
}

impl<AllTypes> FunctionResult for AllTypes
where
    AllTypes: WitStore,
    <AllTypes::Layout as Layout>::Flat: FlatResult,
{
    type ExtraParameter = <<AllTypes::Layout as Layout>::Flat as FlatResult>::ExtraParameter;
    type Output = <<AllTypes::Layout as Layout>::Flat as FlatResult>::Output;
}

pub trait FlatResult {
    type ExtraParameter: FlatLayout;
    type Output: MaybeFlatType;
}

impl FlatResult for HNil {
    type ExtraParameter = HNil;
    type Output = ();
}

impl<AnyFlatType> FlatResult for HList![AnyFlatType]
where
    AnyFlatType: FlatType,
{
    type ExtraParameter = HNil;
    type Output = AnyFlatType;
}

impl<FirstFlatType, SecondFlatType, FlatLayoutTail> FlatResult for HList![FirstFlatType, SecondFlatType, ...FlatLayoutTail]
where
    FirstFlatType: FlatType,
    SecondFlatType: FlatType,
    FlatLayoutTail: FlatLayout,
{
    type ExtraParameter = HList![i32];
    type Output = ();
}

pub trait ResultStorage {
    type OutputFor<Results>: FlatLayout
    where
        Results: WitStore;

    fn lower_result<Results, Instance>(
        self,
        result: Results,
        memory: &mut Memory<'_, Instance>,
    ) -> Result<Self::OutputFor<Results>, RuntimeError>
    where
        Results: WitStore,
        Instance: InstanceWithMemory,
        <Instance::Runtime as Runtime>::Memory: RuntimeMemory<Instance>;
}

impl ResultStorage for () {
    type OutputFor<Results> = <Results::Layout as Layout>::Flat
    where
        Results: WitStore;

    fn lower_result<Results, Instance>(
        self,
        result: Results,
        memory: &mut Memory<'_, Instance>,
    ) -> Result<Self::OutputFor<Results>, RuntimeError>
    where
        Results: WitStore,
        Instance: InstanceWithMemory,
        <Instance::Runtime as Runtime>::Memory: RuntimeMemory<Instance>,
    {
        result.lower(memory)
    }
}

impl ResultStorage for GuestPointer {
    type OutputFor<Results> = HNil
    where
        Results: WitStore;

    fn lower_result<Results, Instance>(
        self,
        result: Results,
        memory: &mut Memory<'_, Instance>,
    ) -> Result<Self::OutputFor<Results>, RuntimeError>
    where
        Results: WitStore,
        Instance: InstanceWithMemory,
        <Instance::Runtime as Runtime>::Memory: RuntimeMemory<Instance>,
    {
        result.store(memory, self)?;

        Ok(HNil)
    }
}
