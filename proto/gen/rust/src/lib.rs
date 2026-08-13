pub mod cognyx {
    pub mod common {
        pub mod v1 {
            tonic::include_proto!("cognyx.common.v1");
        }
    }
    pub mod bus {
        pub mod v1 {
            tonic::include_proto!("cognyx.bus.v1");
        }
    }
    pub mod services {
        pub mod core {
            pub mod v1 {
                tonic::include_proto!("cognyx.services.core.v1");
            }
        }
        pub mod ai {
            pub mod v1 {
                tonic::include_proto!("cognyx.services.ai.v1");
            }
        }
        pub mod security {
            pub mod v1 {
                tonic::include_proto!("cognyx.services.security.v1");
            }
        }
        pub mod runtime {
            pub mod v1 {
                tonic::include_proto!("cognyx.services.runtime.v1");
            }
        }
        pub mod agent {
            pub mod v1 {
                tonic::include_proto!("cognyx.services.agent.v1");
            }
        }
        pub mod capability {
            pub mod v1 {
                tonic::include_proto!("cognyx.services.capability.v1");
            }
        }
    }
}
