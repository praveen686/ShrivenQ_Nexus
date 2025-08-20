# ShrivenQ Documentation

## Overview

This documentation covers the comprehensive architecture, development practices, and operational procedures for ShrivenQ, an ultra-low latency quantitative trading platform built in Rust.

## 📚 Documentation Structure

### 🏗️ Architecture & Vision

#### [Vision](architecture/VISION.md)
Ultimate vision document for ShrivenQ as the world's most advanced ultra-low latency trading platform, covering comprehensive multi-asset trading capabilities, GPU acceleration, and sub-100μs execution targets.

**Key Topics:**
- Executive vision for ultra-low latency trading
- Multi-asset trading architecture (Crypto/Equity/Options/Futures) 
- CUDA acceleration specifications
- Local exchange simulation capabilities
- Performance specifications and competitive advantages

#### [Implementation Architecture](architecture/IMPLEMENTATION.md) 
Production-ready architecture documentation reflecting the ACTUAL implemented system with pragmatic roadmap for expansion and sub-100μs order execution targets.

**Key Topics:**
- Current implementation status (Phase 1 completed)
- Memory architecture (lock-free, NUMA-aware, slab allocators)
- Performance metrics and latency budgets
- Phase-based implementation roadmap
- Critical success factors for HFT systems

#### [ShrivenQuant Learnings](architecture/SHRIVENQUANT_LEARNINGS.md)
Comprehensive analysis of the previous ShrivenQuant codebase extracting battle-tested insights, architectural patterns, and lessons learned for building ShrivenQ as a significant evolution.

**Key Topics:**
- Proven patterns to preserve (service architecture, lock-free structures)
- Critical issues to fix (naming inconsistencies, missing GPU acceleration)
- Performance lessons and optimization strategies
- Best practices for financial system development
- Migration priorities and improvement roadmap

### 🎨 Standards & Conventions

#### [Development Standards](standards/DEVELOPMENT_STANDARDS.md)
Comprehensive development standards for ShrivenQ derived from the clippy configuration crisis, ShrivenQuant learnings, and HFT best practices. Covers safe vs unsafe system requirements, performance standards, and complete development workflow.

**Key Topics:**
- Safe vs unsafe HFT system standards and performance targets
- Code quality standards (including clippy crisis lessons)
- Build system architecture and sequential validation
- Performance requirements and monitoring standards
- Safety and testing standards for unsafe code
- Naming conventions and documentation standards
- Error handling and review compliance requirements

#### [Naming Conventions](standards/NAMING_CONVENTIONS.md)
Ultra-comprehensive style guide ensuring consistent naming across the entire codebase, from Rust code to infrastructure, databases, and deployment configurations. *(Integrated as subsection of Development Standards)*

**Key Topics:**
- Project structure and directory naming
- Rust code conventions (types, functions, modules)
- Network services and environment variables
- Database and storage naming patterns
- Testing, build, and tooling conventions

### 🛡️ Development Guidelines

#### [Comprehensive Development Standards](standards/DEVELOPMENT_STANDARDS.md)
**PRIMARY REFERENCE** - Consolidated development standards integrating all learnings from the clippy configuration crisis, ShrivenQuant analysis, and HFT best practices. This 1700+ line document covers:
- Safe vs unsafe system standards and performance targets
- Code quality standards (including clippy crisis lessons)
- Build system architecture and sequential validation
- Performance requirements and monitoring standards
- Safety and testing standards for unsafe code
- Naming conventions and documentation standards
- Error handling and review compliance requirements
- Troubleshooting guides and documentation change tracking

#### [Unsafe Code Standards](standards/UNSAFE_CODE_STANDARDS.md)
**MANDATORY STANDARDS** for unsafe code implementation, combining CTO requirements, implementation verification, and enforcement rules. Serves as both the historical record and the compliance standard for feature-gated unsafe architecture.

**Key Topics:**
- CTO's mandatory requirements and implementation status
- Feature-gated unsafe architecture (`hft-unsafe` flag) enforcement
- Required safety documentation standards
- Memory management patterns (lock-free, NUMA-aware)
- Mandatory testing requirements (Miri, Loom, AddressSanitizer)
- Performance justification requirements
- Compliance with industry standards (crossbeam/parking_lot patterns)

## Documentation Health

For current documentation metrics and health status, see [Documentation Health](metrics/documentation-health.md).

## Quick Reference

### Essential Commands

```bash
# Safe build (default)
cargo build

# High-performance build (opt-in unsafe features)
cargo build --features hft-unsafe

# Comprehensive validation
./scripts/build/build_strict_sequential.sh

# Development iteration
./scripts/build/build_development_quick.sh
```

### Key Architecture Decisions

1. **Safe by Default**: All unsafe code is feature-gated behind `hft-unsafe`
2. **Workspace Lint Tables**: Centralized configuration in `Cargo.toml` instead of CLI flags
3. **Sequential Build Validation**: Each stage gates the next for clear failure attribution
4. **Documentation-First**: Every unsafe operation requires comprehensive safety documentation

## Documentation Standards

### Naming Conventions

- **Files**: `kebab-case-with-hyphens.md`
- **Directories**: `lowercase-with-hyphens/`
- **Section Headers**: `Title Case`
- **Code Examples**: Annotated with `✅ RECOMMENDED` or `❌ PROBLEMATIC`

### Update Schedule

- **Development docs**: Updated with each major architectural change
- **Build system docs**: Reviewed monthly, updated quarterly
- **Safety guidelines**: Reviewed with each Rust release
- **Performance docs**: Updated with benchmark results

## Historical Context

### The Clippy Configuration Crisis (2025-08-20)

ShrivenQ experienced a significant build system crisis when using `clippy::restriction` group globally caused 1000+ contradictory lint errors. The resolution involved:

1. **Root Cause Analysis**: Identified that restriction group contains mutually exclusive lints
2. **Architectural Solution**: Implemented workspace lint tables with priority system
3. **Results**: 94% reduction in clippy issues (1000+ → 158 manageable warnings)
4. **Documentation**: Comprehensive guides to prevent recurrence

This crisis led to the development of the current robust build system architecture and comprehensive documentation standards.

## Contributing to Documentation

### Adding New Documentation

1. **Follow naming conventions** outlined above
2. **Include comprehensive examples** with clear annotations
3. **Document rationale** for architectural decisions
4. **Add to this index** with appropriate categorization

### Review Process

- **Technical accuracy**: All examples must compile and run
- **Clarity**: Documentation should be accessible to team members
- **Completeness**: Cover both successful and failure cases
- **Maintenance**: Include update schedules and ownership

---

**Documentation Version:** 1.0  
**Last Updated:** 2025-08-20  
**Next Review:** 2025-11-20  
**Maintainers:** ShrivenQ Development Team