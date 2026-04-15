$watch_later_video_ids=teamy-mft query --in G:\TeamyYoutube\ "watch later" | % { Split-Path -Parent $_ }
$video_datas=teamy-mft query --in G:\TeamyYoutube\ "fetch_video_data.json$"
$video_data_lookup = New-Object hashtable
$video_datas | % { 
    $video_path=Split-Path $_ -Parent
    $video_data=Get-Content -Raw $_ | ConvertFrom-Json
    $video_data_lookup.Add("$video_path",$video_data)
}
$watch_later_videos=$watch_later_video_ids|%{$video_data_lookup[$_]}
