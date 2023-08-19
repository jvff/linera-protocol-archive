mod results;

use {
    self::results::ResultStorage,
    crate::{
        memory_layout::FlatLayout, primitive_types::FlatType, util::Split, GuestPointer,
        InstanceWithMemory, Layout, Memory, Runtime, RuntimeError, RuntimeMemory, WitLoad,
        WitStore, WitType,
    },
    frunk::HList,
    std::ops::Add,
};

/// TODO
pub trait ExportTo<Instance> {
    /// TODO
    fn export_to(instance: &mut Instance) -> Result<(), RuntimeError>;
}

/// TODO
pub trait ExportFunction<Handler, Parameters, Results> {
    /// TODO
    fn export(
        &mut self,
        module_name: &str,
        function_name: &str,
        handler: Handler,
    ) -> Result<(), RuntimeError>;
}

/// TODO
pub trait ExportedFunctionInterface {
    /// TODO
    type HostParameters: WitType;
    /// TODO
    type HostResults: WitStore;
    /// TODO
    type FlatInterface: FlatExportFunctionInterface<
        FlatParameters = <<Self::HostParameters as WitType>::Layout as Layout>::Flat,
        ResultStorage = Self::ResultStorage,
    >;
    /// TODO
    type GuestParameters;
    /// TODO
    type GuestResults;
    /// TODO
    type ResultStorage: ResultStorage<OutputFor<Self::HostResults> = Self::GuestResults>;

    /// TODO
    fn lift_parameters<Instance>(
        guest_parameters: Self::GuestParameters,
        memory: &Memory<'_, Instance>,
    ) -> Result<(Self::HostParameters, Self::ResultStorage), RuntimeError>
    where
        Instance: InstanceWithMemory,
        <Instance::Runtime as Runtime>::Memory: RuntimeMemory<Instance>;

    /// TODO
    fn lower_results<Instance>(
        results: Self::HostResults,
        result_storage: Self::ResultStorage,
        memory: &mut Memory<'_, Instance>,
    ) -> Result<Self::GuestResults, RuntimeError>
    where
        Instance: InstanceWithMemory,
        <Instance::Runtime as Runtime>::Memory: RuntimeMemory<Instance>;
}

impl<Parameters, Results> ExportedFunctionInterface for (Parameters, Results)
where
    Parameters: WitLoad,
    Results: WitStore,
    HList![
        <Parameters::Layout as Layout>::Flat,
        <Results::Layout as Layout>::Flat,
    ]: FlatExportFunctionInterface<FlatParameters = <Parameters::Layout as Layout>::Flat>,
    <() as WitType>::Layout: Layout<Flat = frunk::HNil>,
{
    type HostParameters = Parameters;
    type HostResults = Results;
    type FlatInterface = HList![
        <Parameters::Layout as Layout>::Flat,
        <Results::Layout as Layout>::Flat,
    ];
    type GuestParameters = <Self::FlatInterface as FlatExportFunctionInterface>::GuestParameters;
    type GuestResults = <<HList![
        <Parameters::Layout as Layout>::Flat,
        <Results::Layout as Layout>::Flat,
    ] as FlatExportFunctionInterface>::ResultStorage as ResultStorage>::OutputFor<
        Self::HostResults,
    >;
    type ResultStorage = <HList![
        <Parameters::Layout as Layout>::Flat,
        <Results::Layout as Layout>::Flat,
    ] as FlatExportFunctionInterface>::ResultStorage;

    fn lift_parameters<Instance>(
        guest_parameters: Self::GuestParameters,
        memory: &Memory<'_, Instance>,
    ) -> Result<(Self::HostParameters, Self::ResultStorage), RuntimeError>
    where
        Instance: InstanceWithMemory,
        <Instance::Runtime as Runtime>::Memory: RuntimeMemory<Instance>,
    {
        Self::FlatInterface::lift_parameters(guest_parameters, memory)
    }

    fn lower_results<Instance>(
        results: Self::HostResults,
        result_storage: Self::ResultStorage,
        memory: &mut Memory<'_, Instance>,
    ) -> Result<Self::GuestResults, RuntimeError>
    where
        Instance: InstanceWithMemory,
        <Instance::Runtime as Runtime>::Memory: RuntimeMemory<Instance>,
    {
        result_storage.lower_result(results, memory)
    }
}

pub trait FlatExportFunctionInterface {
    type FlatParameters: FlatLayout;
    type GuestParameters: FlatLayout;
    type ResultStorage: ResultStorage;

    fn lift_parameters<Instance, HostParameters>(
        guest_parameters: Self::GuestParameters,
        memory: &Memory<'_, Instance>,
    ) -> Result<(HostParameters, Self::ResultStorage), RuntimeError>
    where
        HostParameters: WitLoad,
        HostParameters::Layout: Layout<Flat = Self::FlatParameters>,
        Instance: InstanceWithMemory,
        <Instance::Runtime as Runtime>::Memory: RuntimeMemory<Instance>;
}

macro_rules! direct_interface {
    ($( $types:ident ),* $(,)*) => { direct_interface!(| $( $types ),*); };

    ($( $types:ident ),* |) => { direct_interface!(@generate $( $types ),*); };

    ($( $types:ident ),* | $next_type:ident $(, $queued_types:ident )*) => {
        direct_interface!(@generate $( $types ),*);
        direct_interface!($( $types, )* $next_type | $( $queued_types ),*);
    };

    (@generate $( $types:ident ),*) => {
        direct_interface!(@generate $( $types ),* =>);
        direct_interface!(@generate $( $types ),* => FlatResult);
    };

    (@generate $( $types:ident ),* => $( $flat_result:ident )?) => {
        impl<$( $types, )* $( $flat_result )*> FlatExportFunctionInterface
            for HList![HList![$( $types, )*], HList![$( $flat_result )*]]
        where
            HList![$( $types, )*]: FlatLayout,
            $( $flat_result: FlatType, )*
        {
            type FlatParameters = HList![$( $types, )*];
            type GuestParameters = HList![$( $types, )*];
            type ResultStorage = ();

            fn lift_parameters<Instance, HostParameters>(
                guest_parameters: Self::GuestParameters,
                memory: &Memory<'_, Instance>,
            ) -> Result<(HostParameters, Self::ResultStorage), RuntimeError>
            where
                HostParameters: WitLoad,
                HostParameters::Layout: Layout<Flat = Self::FlatParameters>,
                Instance: InstanceWithMemory,
                <Instance::Runtime as Runtime>::Memory: RuntimeMemory<Instance>,
            {
                let parameters = HostParameters::lift_from(guest_parameters, memory)?;

                Ok((parameters, ()))
            }
        }
    };
}

direct_interface!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

macro_rules! indirect_results {
    ($( $types:ident ),* $(,)*) => { indirect_results!(| $( $types ),*); };

    ($( $types:ident ),* |) => { indirect_results!(@generate $( $types ),*); };

    ($( $types:ident ),* | $next_type:ident $(, $queued_types:ident )*) => {
        indirect_results!(@generate $( $types ),*);
        indirect_results!($( $types, )* $next_type | $( $queued_types ),*);
    };

    (@generate $( $types:ident ),*) => {
        impl<$( $types, )* Y, Z, Tail> FlatExportFunctionInterface
            for HList![HList![$( $types, )*], HList![Y, Z, ...Tail]]
        where
            HList![$( $types, )*]: FlatLayout + Add<HList![i32]>,
            <HList![$( $types, )*] as Add<HList![i32]>>::Output:
                FlatLayout + Split<HList![$( $types, )*], Remainder = HList![i32]>,
        {
            type FlatParameters = HList![$( $types, )*];
            type GuestParameters = <Self::FlatParameters as Add<HList![i32]>>::Output;
            type ResultStorage = GuestPointer;

            fn lift_parameters<Instance, Parameters>(
                guest_parameters: Self::GuestParameters,
                memory: &Memory<'_, Instance>,
            ) -> Result<(Parameters, Self::ResultStorage), RuntimeError>
            where
                Parameters: WitLoad,
                Parameters::Layout: Layout<Flat = Self::FlatParameters>,
                Instance: InstanceWithMemory,
                <Instance::Runtime as Runtime>::Memory: RuntimeMemory<Instance>,
            {
                let (parameters_layout, result_storage_layout) = guest_parameters.split();
                let parameters = Parameters::lift_from(parameters_layout, memory)?;
                let result_storage = Self::ResultStorage::lift_from(result_storage_layout, memory)?;

                Ok((parameters, result_storage))
            }
        }
    };
}

indirect_results!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);

macro_rules! indirect_parameters {
    (=> $( $flat_result:ident )? ) => {
        impl<A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, Tail $(, $flat_result )*>
            FlatExportFunctionInterface
            for HList![
                HList![A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, ...Tail],
                HList![$( $flat_result )*],
            ]
        where
            HList![A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, ...Tail]: FlatLayout,
            $( $flat_result: FlatType, )*
        {
            type FlatParameters = HList![A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, ...Tail];
            type GuestParameters = HList![i32];
            type ResultStorage = ();

            fn lift_parameters<Instance, Parameters>(
                guest_parameters: Self::GuestParameters,
                memory: &Memory<'_, Instance>,
            ) -> Result<(Parameters, Self::ResultStorage), RuntimeError>
            where
                Parameters: WitLoad,
                Parameters::Layout: Layout<Flat = Self::FlatParameters>,
                Instance: InstanceWithMemory,
                <Instance::Runtime as Runtime>::Memory: RuntimeMemory<Instance>,
            {
                let parameters_location = GuestPointer::lift_from(guest_parameters, memory)?;
                let parameters = Parameters::load(memory, parameters_location)?;

                Ok((parameters, ()))
            }
        }
    };
}

indirect_parameters!(=>);
indirect_parameters!(=> Z);

impl<A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, OtherParameters, Y, Z, OtherResults>
    FlatExportFunctionInterface
    for HList![
        HList![A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, ...OtherParameters],
        HList![Y, Z, ...OtherResults],
    ]
where
    HList![A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, ...OtherParameters]: FlatLayout,
    HList![Y, Z, ...OtherResults]: FlatLayout,
{
    type FlatParameters =
        HList![A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, ...OtherParameters];
    type GuestParameters = HList![i32, i32];
    type ResultStorage = GuestPointer;

    fn lift_parameters<Instance, Parameters>(
        guest_parameters: Self::GuestParameters,
        memory: &Memory<'_, Instance>,
    ) -> Result<(Parameters, Self::ResultStorage), RuntimeError>
    where
        Parameters: WitLoad,
        Parameters::Layout: Layout<Flat = Self::FlatParameters>,
        Instance: InstanceWithMemory,
        <Instance::Runtime as Runtime>::Memory: RuntimeMemory<Instance>,
    {
        let (parameters_layout, result_storage_layout) = guest_parameters.split();
        let parameters_location = GuestPointer::lift_from(parameters_layout, memory)?;
        let parameters = Parameters::load(memory, parameters_location)?;
        let result_storage = Self::ResultStorage::lift_from(result_storage_layout, memory)?;

        Ok((parameters, result_storage))
    }
}
