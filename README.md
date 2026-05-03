# il2cpp dumper

Original Implementation: https://github.com/lanylow/honkai-dumper

ZZZ 2.7

CS dump
```cs
// Namespace: Nap.NapECS
public class ComponentMaskFilter
{
	// Fields
	private readonly System.Collections.Generic.List<Nap.NapECS.ComponentMaskFilter.ComplexMask> _complexMasks; // 0x10
	private Nap.NapECS.EcsWorld _cachedWorld; // 0x18
	private System.Collections.Generic.List<Nap.NapECS.ComponentMask> _buffer; // 0x20
	private System.Collections.Generic.List<Nap.NapECS.ComponentMask> _cachedComponentMask; // 0x28
	private Nap.NapECS.ComponentMask _noneMask; // 0x30
	private System.Nullable<uint> _cachedVersion; // 0x50
	private Nap.NapECS.ComponentMask _allMask; // 0x58

	// Methods

	// RVA: 0x18dc2990 VA: 0x198dc2990
	void .ctor() { }

	// RVA: 0x18dc0e50 VA: 0x198dc0e50
	System.Collections.Generic.List<Nap.NapECS.ComponentMask> get_CacheMask() { }

	// RVA: 0x18dc0f10 VA: 0x198dc0f10
	System.Collections.Generic.List<Nap.NapECS.ComponentMask> get_Buffer() { }

	// RVA: 0x18dc0fd0 VA: 0x198dc0fd0
	void MarkCacheDirty() { }

	// RVA: 0x18dc1080 VA: 0x198dc1080
	void ResetCache(Nap.NapECS.EcsWorld world) { }

	// RVA: 0x18dc11b0 VA: 0x198dc11b0
	System.Collections.Generic.List.Enumerator<Nap.NapECS.ComponentMask> GetEnumerator(Nap.NapECS.EcsWorld world) { }

	// RVA: 0x18dc1350 VA: 0x198dc1350
	bool IsPass(in Nap.NapECS.ComponentMask mask) { }

	// RVA: 0x18dc20a0 VA: 0x198dc20a0
	bool IsValid() { }

	// RVA: 0x18dc2680 VA: 0x198dc2680
	Nap.NapECS.ComponentMaskFilter All(in Nap.NapECS.ComponentMask mask) { }
}
```

JSON dump
```json
{
  "ScriptField": [
    {
      "Address": 81192560,
      "Name": "<PrivateImplementationDetails>_E92B39D8233061927D9ACDE54665E68E7535635A",
      "Value": "000000001F0000003B0000005A0000007800000097000000B5000000D4000000F300000011010000300100004E0100006D010000"
    },
    {
      "Address": 81192568,
      "Name": "<PrivateImplementationDetails>_DD3AEFEADB1CD615F3017763F1568179FEE640B0",
      "Value": "000000001F0000003C0000005B0000007900000098000000B6000000D5000000F400000012010000310100004F0100006E010000"
    },
  ],
  "ScriptMethod": [
    {
      "Address": 419295600,
      "Name": "void Locale::.ctor()"
    },
    {
      "Address": 419295616,
      "Name": "string Locale::GetText(string msg)"
    },
  ],
  "ScriptString": [
    {
      "Address": 81200408,
      "Value": "[*]"
    },
    {
      "Address": 81200440,
      "Value": "The input array length must not exceed Int32.MaxValue / {0}. Otherwise BitArray.Length would exceed Int32.MaxValue."
    }
  ]
}
```